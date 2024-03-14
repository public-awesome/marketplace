use anyhow::Error;
use cw_multi_test::AppResponse;

pub fn assert_error(response: Result<AppResponse, Error>, expected: String) {
    assert_eq!(
        response.unwrap_err().source().unwrap().to_string(),
        expected
    );
}
