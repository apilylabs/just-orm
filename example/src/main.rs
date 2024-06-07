// examples/example.rs
use just_orm::{JsonDataStore, JsonDbExtensions, TestData};
fn main() {
    // 디렉터리 생성
    let mut data_store = JsonDataStore::new(Some("test_model"));

    // 데이터 생성
    let data1 = TestData {
        id: "1".to_string(),
        name: "Alice".to_string(),
    };
    let data2 = TestData {
        id: "2".to_string(),
        name: "Bob".to_string(),
    };

    // 모델에 데이터 저장
    data_store.create_model(data1.clone());
    data_store.create_model(data2.clone());

    // ID로 데이터 찾기
    if let Some(found_data) = data_store.find_by_id::<TestData>("1") {
        println!("Found data by id: {:?}", found_data);
    }

    // 모든 데이터 찾기
    let all_data: Vec<TestData> = data_store.find_all();
    println!("All data: {:?}", all_data);

    // 조건에 맞는 데이터 찾기
    let condition = TestData {
        id: "".to_string(),
        name: "Alice".to_string(),
    };
    let filtered_data: Vec<TestData> = data_store.find(&condition);
    println!("Filtered data: {:?}", filtered_data);

    // 데이터 업데이트
    let updated_data = TestData {
        id: "1".to_string(),
        name: "Alice Updated".to_string(),
    };
    data_store.update_by_id("1", updated_data.clone());

    // 업데이트된 데이터 확인
    if let Some(found_data) = data_store.find_by_id::<TestData>("1") {
        println!("Updated data: {:?}", found_data);
    }

    // 데이터 삭제
    // data_store.delete_by_id::<TestData>("2");

    // 삭제 후 모든 데이터 확인
    let all_data_after_delete: Vec<TestData> = data_store.find_all();
    println!("All data after delete: {:?}", all_data_after_delete);
}
