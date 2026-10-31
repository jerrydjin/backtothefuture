use muse::osm;
const XML: &str = include_str!("fixtures/import.osm");
#[test]
fn deterministic_import_filters_access_and_keeps_largest_directed_component() {
    let a = osm::import(XML).unwrap();
    let b = osm::import(XML).unwrap();
    assert_eq!(
        serde_json::to_vec(&a).unwrap(),
        serde_json::to_vec(&b).unwrap()
    );
    assert_eq!(a.nodes.len(), 6);
    assert_eq!(a.edges.len(), 7);
    assert!(a.nodes.iter().all(|n| n.location.lon > -122.415));
    assert_eq!(a.nodes[4].location.lat, 37.754);
    assert_eq!(a.nodes[4].location.lon, -122.408);
    assert_eq!(a.edges[0].lanes, 2);
    assert!((a.edges[0].speed_limit_mps - 13.4112).abs() < 1e-6);
    assert_eq!(a.edges[1].lanes, 1);
    assert!((a.edges[1].speed_limit_mps - 11.176).abs() < 1e-6);
    assert!(a.edges[2].one_way);
    assert!(a.edges[4].one_way);
    assert_eq!(a.nodes[a.edges[4].from].location.lat, 37.752);
    assert_eq!(a.nodes[a.edges[4].to].location.lat, 37.754);
    assert!(!a.network().unwrap().routes().is_empty());
}
#[test]
fn directional_lane_tags_and_missing_geometry_are_handled_explicitly() {
    let xml = XML.replace(
        "<tag k=\"lanes\" v=\"4\"/>",
        "<tag k=\"lanes:forward\" v=\"2\"/><tag k=\"lanes:backward\" v=\"1\"/>",
    );
    let a = osm::import(&xml).unwrap();
    assert!(a.edges[0].one_way && a.edges[1].one_way);
    assert_eq!((a.edges[0].lanes, a.edges[1].lanes), (2, 1));
    assert!(osm::import(&XML.replace("ref=\"1\"", "ref=\"999\"")).is_err());
    assert!(osm::import("<!DOCTYPE osm><osm/>").is_err());
    assert!(osm::import("<osm/>").is_err());
}
#[test]
fn importer_cli_writes_exact_format_and_rejects_bad_usage() {
    let dir = std::env::temp_dir().join(format!("muse-osm-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("map.json");
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_muse"))
        .args(["import-osm", "tests/fixtures/import.osm", "--out"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        std::fs::read(&out).unwrap(),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&osm::import(XML).unwrap()).unwrap()
        )
        .as_bytes()
    );
    assert!(!std::process::Command::new(env!("CARGO_BIN_EXE_muse"))
        .arg("import-osm")
        .output()
        .unwrap()
        .status
        .success());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn distinct_roundabout_halves_are_not_mistaken_for_reverse_carriageways() {
    let xml = r#"<osm version="0.6">
    <node id="1" lat="37.75" lon="-122.41"><tag k="amenity" v="hospital"/></node>
    <node id="2" lat="37.751" lon="-122.409"/>
    <node id="3" lat="37.752" lon="-122.41"/>
    <node id="4" lat="37.751" lon="-122.411"/>
    <way id="1"><nd ref="1"/><nd ref="2"/><nd ref="3"/><nd ref="4"/><nd ref="1"/>
    <tag k="highway" v="residential"/><tag k="junction" v="roundabout"/></way></osm>"#;
    let map = osm::import(xml).unwrap();
    assert_eq!(map.edges.len(), 2);
    assert!(map.edges.iter().all(|e| e.one_way));
    assert!(!map.network().unwrap().routes().is_empty());
}
