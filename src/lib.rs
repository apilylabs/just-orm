use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

const BASE_DIR: &str = "json-db";

fn create_directory_if_not_exists(dir: &str) {
    if !Path::new(dir).exists() {
        fs::create_dir_all(dir).expect("Failed to create directory");
    }
}

fn read_json_file<T: serde::de::DeserializeOwned>(file_path: &str) -> Option<T> {
    if Path::new(file_path).exists() {
        let mut file = File::open(file_path).expect("Failed to open file");
        let mut content = String::new();
        file.read_to_string(&mut content)
            .expect("Failed to read file");
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

fn write_json_file<T: serde::Serialize>(file_path: &str, data: &T) {
    let content = serde_json::to_string_pretty(data).expect("Failed to serialize data");
    let mut file = File::create(file_path).expect("Failed to create file");
    file.write_all(content.as_bytes())
        .expect("Failed to write file");
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TestData {
    pub id: String,
    pub name: String,
}

pub struct JsonDataStore {
    base_path: String,
    current_model_name: Option<String>,
}

impl JsonDataStore {
    pub fn new(model_name: Option<&str>) -> Self {
        let base_path = match model_name {
            Some(name) => {
                let path = format!("{}/{}", BASE_DIR, name);
                create_directory_if_not_exists(&path);
                path
            }
            None => {
                create_directory_if_not_exists(BASE_DIR);
                BASE_DIR.to_string()
            }
        };

        Self {
            base_path,
            current_model_name: model_name.map(|name| name.to_string()),
        }
    }

    pub fn model(&mut self, model_name: &str) -> &mut Self {
        self.current_model_name = Some(model_name.to_string());
        create_directory_if_not_exists(&format!("{}/{}", BASE_DIR, model_name));
        self
    }

    fn get_model_path(&self, model_name: &str) -> String {
        format!("{}/{}", BASE_DIR, model_name)
    }

    fn get_file_path(&self, id: &str) -> String {
        let model_name = self
            .current_model_name
            .as_ref()
            .expect("Model name is not specified");
        format!("{}/{}.json", self.get_model_path(model_name), id)
    }

    fn get_all_files(&self) -> Vec<String> {
        let model_name = self
            .current_model_name
            .as_ref()
            .expect("Model name is not specified");
        let model_path = self.get_model_path(model_name);
        create_directory_if_not_exists(&model_path);
        fs::read_dir(&model_path)
            .expect("Failed to read directory")
            .filter_map(|entry| {
                let entry = entry.expect("Failed to read directory entry");
                if entry.path().extension().map_or(false, |ext| ext == "json") {
                    Some(entry.path().display().to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn create_model<T: serde::Serialize + JsonDbExtensions>(&self, data: T) {
        let id = data.get_id().to_string();
        self.create(&id, data);
    }

    pub fn create<T: serde::Serialize>(&self, id: &str, data: T) {
        let file_path = self.get_file_path(id);
        write_json_file(&file_path, &data);
    }

    pub fn find_by_id<T: serde::de::DeserializeOwned>(&self, id: &str) -> Option<T> {
        let file_path = self.get_file_path(id);
        read_json_file(&file_path)
    }

    pub fn update_by_id<T: serde::de::DeserializeOwned + serde::Serialize + JsonDbExtensions>(
        &self,
        id: &str,
        data: T,
    ) {
        let file_path = self.get_file_path(id);
        let existing_data: Option<T> = self.find_by_id(id);
        if let Some(mut existing_data) = existing_data {
            existing_data.update(data);
            write_json_file(&file_path, &existing_data);
        }
    }

    pub fn delete_by_id<T: JsonDbExtensions>(&self, id: &str) {
        let file_path = self.get_file_path(id);
        if Path::new(&file_path).exists() {
            fs::remove_file(file_path).expect("Failed to delete file");
        }
    }

    pub fn find_all<T: serde::de::DeserializeOwned>(&self) -> Vec<T> {
        let files = self.get_all_files();
        files
            .into_iter()
            .filter_map(|file| read_json_file::<T>(&file))
            .collect()
    }

    pub fn find<T: serde::de::DeserializeOwned + JsonDbExtensions>(&self, condition: &T) -> Vec<T> {
        self.find_all::<T>()
            .into_iter()
            .filter(|item| item.matches_condition(condition))
            .collect()
    }

    pub fn find_one<T: serde::de::DeserializeOwned + JsonDbExtensions>(
        &self,
        condition: &T,
    ) -> Option<T> {
        self.find_all::<T>()
            .into_iter()
            .find(|item| item.matches_condition(condition))
    }

    pub fn count<T: serde::de::DeserializeOwned + JsonDbExtensions>(&self, condition: &T) -> usize {
        self.find(condition).len()
    }

    pub fn update_many<T: serde::de::DeserializeOwned + serde::Serialize + JsonDbExtensions>(
        &self,
        condition: &T,
        data: T,
    ) {
        let items = self.find(condition);
        for item in items {
            let id = item.get_id().to_string();
            self.update_by_id(&id, data.clone());
        }
    }

    pub fn delete_many<T: serde::de::DeserializeOwned + JsonDbExtensions>(&self, condition: &T) {
        let items = self.find(condition);
        for item in items {
            let id = item.get_id().to_string();
            self.delete_by_id::<T>(&id);
        }
    }

    pub fn push<T: serde::de::DeserializeOwned + serde::Serialize + JsonDbExtensions>(
        &self,
        condition: &T,
        array_path: &str,
        element: T,
    ) {
        let items = self.find(condition);
        for item in items {
            let id = item.get_id().to_string();
            let mut data: T = self.find_by_id(&id).expect("Failed to find item by id");
            data.push_to_array(array_path, element.clone());
            self.update_by_id(&id, data);
        }
    }

    pub fn pull<T: serde::de::DeserializeOwned + serde::Serialize + JsonDbExtensions>(
        &self,
        condition: &T,
        array_path: &str,
        pull_condition: &T,
    ) {
        let items = self.find(condition);
        for item in items {
            let id = item.get_id().to_string();
            let mut data: T = self.find_by_id(&id).expect("Failed to find item by id");
            data.pull_from_array(array_path, pull_condition);
            self.update_by_id(&id, data);
        }
    }

    pub fn update_array<T: serde::de::DeserializeOwned + serde::Serialize + JsonDbExtensions>(
        &self,
        condition: &T,
        array_path: &str,
        array_condition: &T,
        updates: T,
    ) {
        let items = self.find(condition);
        for item in items {
            let id = item.get_id().to_string();
            let mut data: T = self.find_by_id(&id).expect("Failed to find item by id");
            data.update_array(array_path, array_condition, updates.clone());
            self.update_by_id(&id, data);
        }
    }
}

pub trait JsonDbExtensions: Serialize + for<'de> Deserialize<'de> + Clone {
    fn get_id(&self) -> &str;
    fn update(&mut self, other: Self);
    fn matches_condition(&self, condition: &Self) -> bool;
    fn push_to_array(&mut self, array_path: &str, element: Self);
    fn pull_from_array(&mut self, array_path: &str, condition: &Self);
    fn update_array(&mut self, array_path: &str, condition: &Self, updates: Self);
}

impl JsonDbExtensions for TestData {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn update(&mut self, other: Self) {
        self.name = other.name;
    }

    fn matches_condition(&self, condition: &Self) -> bool {
        self.name == condition.name
    }

    fn push_to_array(&mut self, _array_path: &str, _element: Self) {
        // No-op for this simple data type
    }

    fn pull_from_array(&mut self, _array_path: &str, _condition: &Self) {
        // No-op for this simple data type
    }

    fn update_array(&mut self, _array_path: &str, _condition: &Self, _updates: Self) {
        // No-op for this simple data type
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_directory() {
        let _ = fs::remove_dir_all(BASE_DIR); // Remove any existing test directory
        create_directory_if_not_exists(BASE_DIR); // Recreate base directory
    }

    #[test]
    fn test_create_directory_if_not_exists() {
        setup_test_directory();
        let test_dir = format!("{}/test_model", BASE_DIR);
        create_directory_if_not_exists(&test_dir);
        assert!(Path::new(&test_dir).exists());
    }

    #[test]
    fn test_read_write_json_file() {
        setup_test_directory();
        let file_path = format!("{}/test.json", BASE_DIR);
        let test_data = TestData {
            id: "1".to_string(),
            name: "Test".to_string(),
        };

        write_json_file(&file_path, &test_data);
        let read_data: Option<TestData> = read_json_file(&file_path);
        assert_eq!(read_data, Some(test_data));
    }

    #[test]
    fn test_create_and_find_by_id() {
        setup_test_directory();
        let mut store = JsonDataStore::new(Some("test_model"));
        let test_data = TestData {
            id: "1".to_string(),
            name: "Test".to_string(),
        };

        store.create_model(test_data.clone());
        let found_data: Option<TestData> = store.find_by_id("1");
        assert_eq!(found_data, Some(test_data));
    }

    #[test]
    fn test_update_by_id() {
        setup_test_directory();
        let mut store = JsonDataStore::new(Some("test_model"));
        let test_data = TestData {
            id: "1".to_string(),
            name: "Test".to_string(),
        };

        store.create_model(test_data.clone());

        let updated_data = TestData {
            id: "1".to_string(),
            name: "Updated Test".to_string(),
        };
        store.update_by_id("1", updated_data.clone());

        let found_data: Option<TestData> = store.find_by_id("1");
        assert_eq!(found_data, Some(updated_data));
    }

    #[test]
    fn test_delete_by_id() {
        setup_test_directory();
        let mut store = JsonDataStore::new(Some("test_model"));
        let test_data = TestData {
            id: "1".to_string(),
            name: "Test".to_string(),
        };

        store.create_model(test_data.clone());
        store.delete_by_id::<TestData>("1");

        let found_data: Option<TestData> = store.find_by_id("1");
        assert!(found_data.is_none());
    }

    #[test]
    fn test_find_all() {
        setup_test_directory();
        let mut store = JsonDataStore::new(Some("test_model"));
        let test_data1 = TestData {
            id: "1".to_string(),
            name: "Test1".to_string(),
        };
        let test_data2 = TestData {
            id: "2".to_string(),
            name: "Test2".to_string(),
        };

        store.create_model(test_data1.clone());
        store.create_model(test_data2.clone());

        let all_data: Vec<TestData> = store.find_all();
        assert!(all_data.contains(&test_data1));
        assert!(all_data.contains(&test_data2));
    }

    #[test]
    fn test_find() {
        setup_test_directory();
        let mut store = JsonDataStore::new(Some("test_model"));
        let test_data1 = TestData {
            id: "1".to_string(),
            name: "FindMe".to_string(),
        };
        let test_data2 = TestData {
            id: "2".to_string(),
            name: "DontFindMe".to_string(),
        };

        store.create_model(test_data1.clone());
        store.create_model(test_data2.clone());

        let condition = TestData {
            id: "".to_string(),
            name: "FindMe".to_string(),
        };

        let found_data: Vec<TestData> = store.find(&condition);
        assert_eq!(found_data.len(), 1);
        assert_eq!(found_data[0], test_data1);
    }
}
