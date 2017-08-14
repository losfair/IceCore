use prefix_tree::PrefixTree;

#[test]
fn test_prefix_tree() {
    let mut pt: PrefixTree<String, i32> = PrefixTree::new();

    let key_seq_1 = vec![
        "a".to_string(),
        "b".to_string(),
        "c".to_string()
    ];
    pt.insert(key_seq_1.as_slice(), 1);

    let key_seq_2 = vec![
        "a".to_string(),
        "b".to_string(),
        "d".to_string()
    ];
    pt.insert(key_seq_2.as_slice(), 2);

    let key_seq_3 = vec![
        "a".to_string(),
        "c".to_string(),
        "d".to_string()
    ];
    pt.insert(key_seq_3.as_slice(), 3);

    let key_seq_4 = vec![
        "a".to_string(),
        "_".to_string(),
        "_".to_string()
    ];
    pt.insert(key_seq_4.as_slice(), 4);

    assert!(pt.find(key_seq_1.as_slice(), None).unwrap() == 1);
    assert!(pt.find(key_seq_2.as_slice(), None).unwrap() == 2);
    assert!(pt.find(key_seq_3.as_slice(), None).unwrap() == 3);
    assert!(pt.find(vec![
        "a".to_string(),
        "n".to_string(),
        "d".to_string()
    ].as_slice(), Some(&"b".to_string())).unwrap() == 2);
    assert!(pt.find(vec![
        "a".to_string(),
        "m".to_string(),
        "n".to_string()
    ].as_slice(), Some(&"_".to_string())).unwrap() == 4);
}
