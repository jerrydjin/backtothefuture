#!/usr/bin/env python3
"""Build or check Phase 15 SAM evidence, with no network access.

Generation requires a previously downloaded Hugging Face snapshot and the
original photograph. --check needs Pillow, but neither torch nor transformers.
Missing evidence is an error; reference measurements are never substituted.
"""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[1]
MODEL_ID = "facebook/sam2.1-hiera-tiny"
REVISION = "de431c40"
IMAGE_NAME = "fallen-tree-a36.jpg"
IMAGE_SHA256 = "07030ee22246306d776c80c74defe29c214914eac81cb2a7fbaae17b3997e55d"
POINTS = [[460, 140], [300, 215], [150, 275]]
SOURCE = (
    "https://commons.wikimedia.org/wiki/"
    "File:Fallen_tree_blocking_A36_-_geograph.org.uk_-_3794373.jpg"
)


def require(ok, message):
    if not ok:
        raise ValueError(message)


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def geometry(mask):
    """Pixel bounds are inclusive, in original-image coordinates."""
    bounds = mask.getbbox()
    require(bounds is not None, "SAM returned an empty mask")
    histogram = mask.histogram()
    require(sum(histogram[1:255]) == 0, "mask must contain only 0 and 255")
    x0, y0, x1, y1 = bounds
    return [x0, y0, x1 - 1, y1 - 1], histogram[255] / (mask.width * mask.height)


def check(directory):
    from PIL import Image

    for name in [IMAGE_NAME, "mask.png", "sam.json"]:
        require((directory / name).is_file(), f"missing genuine SAM evidence: {directory / name}")
    data = json.loads((directory / "sam.json").read_text())
    require(isinstance(data, dict), "sam.json must be an object")
    require(data.get("version") == 1, "unsupported sam.json version")
    require(data.get("model_id") == MODEL_ID, "wrong SAM model")
    revision = data.get("hf_revision", "")
    require(isinstance(revision, str) and re.fullmatch(r"[0-9a-f]{40}", revision)
            and revision.startswith(REVISION), "wrong or unresolved HF revision")
    require(data.get("prompt_points") == POINTS, "positive prompt points changed")
    require(data.get("prompt_labels") == [1, 1, 1], "positive prompt labels changed")
    require(data.get("device") == "cpu", "SAM must run on CPU")
    require(data.get("dtype") == "float32" and data.get("seed") == 0
            and data.get("torch_threads") == 1, "inference settings changed")
    require(data.get("multimask_output") is False, "unexpected mask selection")
    require(data.get("mask_threshold") == 0.0, "unexpected mask threshold")
    for key in ["torch_version", "transformers_version"]:
        require(isinstance(data.get(key), str) and bool(data[key]), f"missing {key}")
    scores = data.get("iou_scores")
    require(isinstance(scores, list) and len(scores) == 1
            and all(type(v) in (int, float) and math.isfinite(v) for v in scores),
            "invalid predicted IoU scores")
    require(data.get("source_url") == SOURCE, "wrong photograph source")
    require(data.get("image_sha256") == IMAGE_SHA256 == sha256(directory / IMAGE_NAME),
            "photograph sha256 mismatch")
    require(data.get("mask_sha256") == sha256(directory / "mask.png"),
            "mask sha256 mismatch")
    with Image.open(directory / IMAGE_NAME) as photograph, Image.open(directory / "mask.png") as mask:
        require(mask.format == "PNG" and mask.mode == "L", "mask must be an 8-bit grayscale PNG")
        require(mask.size == photograph.size, "mask and photograph sizes differ")
        require(data.get("image_size_wh") == list(photograph.size), "image dimensions mismatch")
        bbox, area = geometry(mask)
        require(data.get("bbox_xyxy_inclusive") == bbox, "mask bounding box mismatch")
        cached_area = data.get("area_fraction")
        require(type(cached_area) in (int, float)
                and math.isfinite(cached_area) and abs(cached_area - area) <= 1e-12,
                "mask area fraction mismatch")
    return data


def generate(directory):
    # Set these before importing Hugging Face libraries. Both the snapshot lookup
    # and from_pretrained also explicitly forbid downloads.
    os.environ["HF_HUB_OFFLINE"] = "1"
    os.environ["TRANSFORMERS_OFFLINE"] = "1"
    from PIL import Image
    import torch
    import transformers
    from huggingface_hub import snapshot_download
    from huggingface_hub.constants import HF_HUB_CACHE
    from transformers import Sam2Model, Sam2Processor

    image_path = directory / IMAGE_NAME
    require(image_path.is_file(), f"missing original photograph: {image_path}")
    require(sha256(image_path) == IMAGE_SHA256, "photograph sha256 mismatch")
    require(not any((directory / name).exists() for name in ["mask.png", "sam.json"]),
            "refusing to overwrite evidence; use a new output directory")
    # Offline Hub lookup requires a full commit or a cached ref. A snapshot
    # downloaded using `main` need not have a ref for the brief's short SHA.
    # Resolve only that fixed prefix, never whichever commit `main` now names.
    snapshots = Path(HF_HUB_CACHE) / "models--facebook--sam2.1-hiera-tiny" / "snapshots"
    matches = [p for p in snapshots.glob(f"{REVISION}*")
               if p.is_dir() and re.fullmatch(r"[0-9a-f]{40}", p.name)]
    require(len(matches) == 1, f"need exactly one cached full revision starting with {REVISION}")
    snapshot = Path(snapshot_download(MODEL_ID, revision=matches[0].name, local_files_only=True))
    revision = snapshot.name
    require(re.fullmatch(r"[0-9a-f]{40}", revision) is not None
            and revision.startswith(REVISION), "cached snapshot has wrong or unresolved revision")
    torch.manual_seed(0)
    torch.set_num_threads(1)
    torch.use_deterministic_algorithms(True)
    processor = Sam2Processor.from_pretrained(str(snapshot), local_files_only=True)
    model = Sam2Model.from_pretrained(
        str(snapshot), local_files_only=True, torch_dtype=torch.float32,
    ).to("cpu").eval()
    with Image.open(image_path) as source:
        image = source.convert("RGB")
    # One image, one object, three positive points. No map or incident input.
    inputs = processor(
        images=image, input_points=[[POINTS]], input_labels=[[[1, 1, 1]]],
        return_tensors="pt",
    ).to("cpu")
    with torch.inference_mode():
        outputs = model(**inputs, multimask_output=False)
    scores = outputs.iou_scores.cpu().reshape(-1).tolist()
    require(len(scores) == 1, "expected one predicted IoU score")
    masks = processor.post_process_masks(
        outputs.pred_masks.cpu(), inputs["original_sizes"].cpu(),
        mask_threshold=0.0, binarize=True,
    )[0]
    require(masks.numel() == image.width * image.height, "expected one original-size mask")
    pixels = masks.reshape(image.height, image.width).to(torch.uint8).mul(255).numpy()
    mask = Image.fromarray(pixels)
    bbox, area = geometry(mask)
    # Validate the complete candidate before publishing either evidence file.
    with tempfile.TemporaryDirectory(prefix="phase15-sam-", dir=directory) as temporary:
        candidate = Path(temporary)
        (candidate / IMAGE_NAME).write_bytes(image_path.read_bytes())
        mask.save(candidate / "mask.png", format="PNG")
        data = {
            "version": 1, "model_id": MODEL_ID, "hf_revision": revision,
            "torch_version": torch.__version__, "transformers_version": transformers.__version__,
            "device": "cpu", "dtype": "float32", "seed": 0, "torch_threads": 1,
            "prompt_points": POINTS, "prompt_labels": [1, 1, 1],
            "multimask_output": False, "mask_threshold": 0.0, "iou_scores": scores,
            "bbox_xyxy_inclusive": bbox, "area_fraction": area,
            "image_size_wh": list(image.size), "source_url": SOURCE,
            "image_sha256": sha256(image_path), "mask_sha256": sha256(candidate / "mask.png"),
        }
        (candidate / "sam.json").write_text(json.dumps(data, indent=2, allow_nan=False) + "\n")
        check(candidate)
        (candidate / "mask.png").replace(directory / "mask.png")
        (candidate / "sam.json").replace(directory / "sam.json")
    return check(directory)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify cached evidence without inference")
    parser.add_argument("--output-dir", type=Path, default=ROOT / "assets" / "phase15")
    args = parser.parse_args()
    try:
        data = check(args.output_dir) if args.check else generate(args.output_dir)
    except (OSError, ValueError, ImportError) as error:
        print(f"Phase 15 SAM blocked: {error}", file=sys.stderr)
        print("Supply the original photograph and genuine cached SAM output before preregistration."
              if args.check else "Install CPU torch/torchvision, transformers and Pillow; supply the photograph and cached "
              "HF snapshot; this script never downloads them.", file=sys.stderr)
        return 1
    print(json.dumps({"check": "OK", "image_sha256": data["image_sha256"],
                      "mask_sha256": data["mask_sha256"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
