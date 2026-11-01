# Phase 15 evidence sources

The required photograph is **Fallen tree blocking A36**, by **David Martin**,
licensed under [CC BY-SA 2.0](https://creativecommons.org/licenses/by-sa/2.0/).
The [Wikimedia Commons source page](https://commons.wikimedia.org/wiki/File:Fallen_tree_blocking_A36_-_geograph.org.uk_-_3794373.jpg)
identifies the original geograph.org.uk photograph, number 3794373.
Its required filename is `fallen-tree-a36.jpg` and its SHA-256 is
`07030ee22246306d776c80c74defe29c214914eac81cb2a7fbaae17b3997e55d`.

The segmentation model is
[`facebook/sam2.1-hiera-tiny`](https://huggingface.co/facebook/sam2.1-hiera-tiny),
licensed under Apache-2.0. The requested HF revision prefix is `de431c40`.
The offline script records the resolved full revision and the actual package
versions in `sam.json`. It runs `Sam2Model` and `Sam2Processor` on CPU with
the three fixed positive points. It uses the single-mask output and a zero
logit threshold. Mask bounds use inclusive original-image pixel coordinates.
There is no acceptance threshold on predicted IoU or mask area.

The photograph and SAM outputs are **not included yet**. No segmentation has
been run in this environment. When generated, `mask.png` is a binary grayscale
segmentation derived from that photograph; retain this attribution and the
source license with the image and derivative. Reference measurements in the
brief are not cached results and must never substitute for inference.
