#[test]
fn test_parse_insert_query() {
    let query = r#"INSERT DATA { <http://a> <http://b> <http://c> }"#;
    let query_upper = query.to_uppercase();
    let keyword = "INSERT DATA";
    
    assert!(query_upper.contains(keyword));
    
    if let Some(start) = query_upper.find(keyword) {
        let search_from = start + keyword.len();
        let remaining = &query[search_from..];
        
        assert!(remaining.contains('{'));
        assert!(remaining.contains('}'));
        
        if let Some(brace_start) = remaining.find('{') {
            let content_start = search_from + brace_start + 1;
            let remaining_after = &query[content_start..];
            
            if let Some(brace_end) = remaining_after.find('}') {
                let content = &query[content_start..content_start + brace_end];
                println!("Content: '{}'", content);
                assert!(!content.trim().is_empty());
            }
        }
    }
}
