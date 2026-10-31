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

#[test]
fn real_potrero_junction_keeps_positive_connectors_and_hospital_area() {
    // Original irregular Potrero spans (100 m then 8 m) made the host spline
    // curl past OSM node 65293538 and its rounded connector have zero length.
    let map = osm::import(include_str!("fixtures/sf-zsfg-junction.osm")).unwrap();
    let net = map.network().unwrap();
    assert!(net.links.iter().all(|l| l.len.is_finite() && l.len > 0.0));
    assert!(!net.routes().is_empty());
    let hospital = &map.nodes[map.hospital_node.expect("hospital is a closed OSM way")];
    assert!((hospital.location.lat - 37.7569).abs() < 0.003);
    assert!((hospital.location.lon + 122.4056).abs() < 0.003);
    assert_eq!(
        serde_json::to_vec(&map).unwrap(),
        serde_json::to_vec(&osm::import(include_str!("fixtures/sf-zsfg-junction.osm")).unwrap())
            .unwrap()
    );
}

#[test]
fn hospital_closed_way_uses_area_centroid_independent_of_vertex_density() {
    // Unequal sampling on the rectangle's east side must not pull its centroid
    // toward the east. The actual nearest retained intersection is node 4.
    let xml = XML.replace("<tag k=\"amenity\" v=\"hospital\"/>", "").replace("</osm>", r#"
    <node id="101" lat="37.7539" lon="-122.4081"/>
    <node id="102" lat="37.7539" lon="-122.4079"/>
    <node id="103" lat="37.7540" lon="-122.4079"/>
    <node id="104" lat="37.7541" lon="-122.4079"/>
    <node id="105" lat="37.7541" lon="-122.4081"/>
    <way id="101"><nd ref="101"/><nd ref="102"/><nd ref="103"/><nd ref="104"/><nd ref="105"/><nd ref="101"/>
    <tag k="amenity" v="hospital"/><tag k="area" v="yes"/></way></osm>"#);
    let map = osm::import(&xml).unwrap();
    let p = &map.nodes[map.hospital_node.unwrap()].location;
    assert_eq!((p.lat, p.lon), (37.754, -122.408));
    assert!(osm::import(&xml.replace("ref=\"105\"", "ref=\"999\""))
        .unwrap_err()
        .contains("hospital references missing node"));
}

#[test]
fn committed_sf_extract_imports_with_the_verified_hospital() {
    let xml = std::fs::read_to_string("maps/sf-zsfg.osm").unwrap();
    let map = osm::import(&xml).unwrap();
    assert_eq!((map.nodes.len(), map.edges.len()), (238, 297));
    assert_eq!(map.hospital_node, Some(4));
    let p = &map.nodes[4].location;
    assert_eq!((p.lat, p.lon), (37.7561683, -122.4067178));
    assert!(map.attribution.as_ref().unwrap().contains("OpenStreetMap"));
    let net = map.network().unwrap();
    assert!(net.links.iter().all(|l| l.len > 0.0 && l.len.is_finite()));
    assert!(!net.routes().is_empty());
}

#[test]
fn sub_car_length_spans_are_removed_before_retaining_drivable_routes() {
    let xml = XML
        .replace(
            "<nd ref=\"3\"/><nd ref=\"4\"/>",
            "<nd ref=\"3\"/><nd ref=\"9\"/><nd ref=\"4\"/>",
        )
        .replace(
            "</osm>",
            r#"<node id="9" lat="37.75399" lon="-122.408"/>
        <way id="11"><nd ref="9"/><nd ref="5"/><tag k="highway" v="service"/></way></osm>"#,
        );
    let map = osm::import(&xml).unwrap();
    let need = muse::world::CAR_LENGTH_M + muse::world::MIN_GAP_M + 0.5 + 0.01;
    assert!(map.edges.iter().all(|e| e.length_m > need));
    let net = map.network().unwrap();
    assert!(net
        .links
        .iter()
        .filter(|l| !l.is_connector())
        .all(|l| l.len > need));
    assert!(!net.routes().is_empty());
}
