use anyhow::Error;
use cw_multi_test::AppResponse;

pub fn assert_error(result: Result<AppResponse, Error>, expected: String) {
    assert_eq!(result.unwrap_err().source().unwrap().to_string(), expected);
}

pub fn find_attrs(response: AppResponse, event_ty: &str, attr_key: &str) -> Vec<String> {
    let mut values: Vec<String> = vec![];
    for event in response.events.iter().filter(|x| x.ty == event_ty) {
        for attr in event.attributes.iter().filter(|x| x.key == attr_key) {
            values.push(attr.value.clone());
        }
    }
    values
}
