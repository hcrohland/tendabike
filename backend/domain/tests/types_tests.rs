use tb_domain::*;

#[test]
fn part_type_all_ordered_nonempty() {
    let types = PartType::all_ordered();
    assert_eq!(types.len(), 29);
}

#[test]
fn part_type_all_ordered_sorted_by_id() {
    let types = PartType::all_ordered();
    for w in types.windows(2) {
        assert!(w[0].id < w[1].id, "expected {} < {}", w[0].id, w[1].id);
    }
}

#[test]
fn parttypeid_get_valid() {
    let bike = PartTypeId::from_id(1).get().unwrap();
    assert_eq!(bike.name, "Bike");
    assert_eq!(bike.id, PartTypeId::from_id(1));
}

#[test]
fn parttypeid_get_invalid() {
    let err = PartTypeId::from_id(999).get().unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
}

#[test]
fn parttypeid_is_main_bike() {
    assert!(PartTypeId::from_id(1).is_main().unwrap());
}

#[test]
fn parttypeid_is_main_shoe() {
    assert!(PartTypeId::from_id(301).is_main().unwrap());
}

#[test]
fn parttypeid_is_main_chain() {
    assert!(!PartTypeId::from_id(4).is_main().unwrap());
}

#[test]
fn parttypeid_is_main_invalid() {
    assert!(PartTypeId::from_id(999).is_main().is_err());
}

#[test]
fn parttypeid_subtypes_bike() {
    let subs = PartTypeId::from_id(1).subtypes();
    assert!(subs.contains(&PartTypeId::from_id(4)));
    assert!(subs.contains(&PartTypeId::from_id(3)));
    assert!(subs.contains(&PartTypeId::from_id(2)));
    assert!(subs.contains(&PartTypeId::from_id(11)));
    assert!(subs.contains(&PartTypeId::from_id(14)));
    assert!(subs.contains(&PartTypeId::from_id(1)));
}

#[test]
fn parttypeid_subtypes_tire_only_self() {
    let subs = PartTypeId::from_id(3).subtypes();
    assert_eq!(subs, vec![PartTypeId::from_id(3)]);
}

#[test]
fn parttypeid_subtypes_shoe_includes_binding() {
    let subs = PartTypeId::from_id(302).subtypes();
    assert!(subs.contains(&PartTypeId::from_id(309)));
}

#[test]
fn parttypeid_act_types_bike() {
    let acts = PartTypeId::from_id(1).act_types();
    assert_eq!(
        acts,
        vec![ActTypeId::from(1), ActTypeId::from(5), ActTypeId::from(9)]
    );
}

#[test]
fn parttypeid_act_types_shoe() {
    let acts = PartTypeId::from_id(301).act_types();
    assert_eq!(
        acts,
        vec![ActTypeId::from(3), ActTypeId::from(4), ActTypeId::from(8)]
    );
}

#[test]
fn activitytype_all_ordered_nonempty() {
    let acts = ActivityType::all_ordered();
    assert_eq!(acts.len(), 11);
}
