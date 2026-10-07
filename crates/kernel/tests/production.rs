use buildercraft_kernel::*;
fn id(n: u128) -> Id {
    Id::new(n).unwrap()
}
#[test]
fn production_allows_multiple_assignments_and_rejects_bad_hierarchy() {
    let mut model = ProductionModel::default();
    for n in 1..=2 {
        model.records.push(ProductionRecord {
            id: id(n),
            kind: ProductionKind::Effect,
            name: format!("Effect {n}"),
            parent: None,
            attributes: Default::default(),
        });
        model.bindings.push(ProductionBinding { object: id(20), record: id(n), role: "scenic".into() });
    }
    model.validate().unwrap();
    model.records[0].parent = Some(id(2));
    model.records[1].parent = Some(id(1));
    assert!(model.validate().is_err());
    model.records[1].parent = None;
    model.links.push(ProductionLink { from: id(1), to: id(3), role: "triggered_by".into() });
    assert!(model.validate().is_err());
}
