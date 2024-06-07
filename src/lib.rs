use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty, Value};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const BASE_DIR: &str = "json-db";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonDatabase<T> {
    base_path: PathBuf,
    current_model_name: Option<String>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> JsonDatabase<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone + Identifiable,
{
    pub fn new(model_name: Option<&str>) -> Self {
        let base_path = if let Some(model_name) = model_name {
            let path = Path::new(BASE_DIR).join(model_name);
            create_directory_if_not_exists(&path);
            path
        } else {
            let path = Path::new(BASE_DIR).to_path_buf();
            create_directory_if_not_exists(&path);
            path
        };

        JsonDatabase {
            base_path,
            current_model_name: model_name.map(String::from),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn model(&mut self, model_name: &str) -> &mut Self {
        self.current_model_name = Some(model_name.to_string());
        create_directory_if_not_exists(&self.get_model_path(model_name));
        self
    }

    fn get_model_path(&self, model_name: &str) -> PathBuf {
        Path::new(BASE_DIR).join(model_name)
    }

    fn get_file_path(&self, id: &str) -> PathBuf {
        let model_name = self
            .current_model_name
            .as_ref()
            .expect("Model name is not specified");
        self.get_model_path(model_name).join(format!("{}.json", id))
    }

    fn get_all_files(&self) -> Vec<String> {
        let model_name = self
            .current_model_name
            .as_ref()
            .expect("Model name is not specified");
        let model_path = self.get_model_path(model_name);
        create_directory_if_not_exists(&model_path);

        fs::read_dir(model_path)
            .expect("Unable to read directory")
            .filter_map(|entry| {
                let entry = entry.expect("Unable to get directory entry");
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "json") {
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                } else {
                    None
                }
            })
            .collect()
    }

    fn matches_condition(&self, item: &Value, condition: &Value) -> bool {
        if !condition.is_object() || condition.is_null() {
            return item == condition;
        }
        if !item.is_object() || item.is_null() {
            return false;
        }

        condition.as_object().unwrap().iter().all(|(key, value)| {
            let keys: Vec<&str> = key.split('.').collect();
            let nested_value = self.get_nested_property(item, &keys);
            if nested_value.is_object() && value.is_object() {
                self.matches_condition(nested_value, value)
            } else {
                nested_value == value
            }
        })
    }

    fn get_nested_property<'a>(&self, obj: &'a Value, keys: &[&str]) -> &'a Value {
        keys.iter()
            .fold(obj, |acc, key| acc.get(*key).unwrap_or(&Value::Null))
    }

    fn set_nested_property(&self, obj: &mut Value, keys: &[&str], value: Value) {
        if keys.len() == 1 {
            obj[keys[0]] = value;
        } else {
            let key = keys[0];
            let next_obj = obj
                .as_object_mut()
                .unwrap()
                .entry(key)
                .or_insert_with(|| Value::Object(Default::default()));
            self.set_nested_property(next_obj, &keys[1..], value);
        }
    }

    fn update_nested_object(&self, target: &mut Value, source: &Value) {
        for (key, value) in source.as_object().unwrap().iter() {
            let keys: Vec<&str> = key.split('.').collect();
            self.set_nested_property(target, &keys, value.clone());
        }
    }

    pub fn create_model(&self, data: T) {
        let id = data.get_id();
        if id.is_empty() {
            panic!("Data must have an id field");
        }
        self.create(&id, data);
    }

    pub fn create(&self, id: &str, data: T) {
        let file_path = self.get_file_path(id);
        write_json_file(&file_path, &data);
    }

    pub fn find_by_id(&self, id: &str) -> Option<T> {
        let file_path = self.get_file_path(id);
        println!("--aaaaaa-{:?}", file_path);
        read_json_file(&file_path)
    }

    pub fn update_by_id(&self, id: &str, data: Value) {
        let file_path = self.get_file_path(id);

        if let Some(mut existing_data) = self.find_by_id(id) {
            let mut existing_json = serde_json::to_value(&existing_data).unwrap();

            self.update_nested_object(&mut existing_json, &data);
            let updated_data: T = serde_json::from_value(existing_json).unwrap();
            write_json_file(&file_path, &updated_data);
        }
    }

    pub fn delete_by_id(&self, id: &str) {
        let file_path = self.get_file_path(id);
        if file_path.exists() {
            fs::remove_file(file_path).expect("Unable to delete file");
        }
    }

    pub fn find_all(&self) -> Vec<T> {
        let files = self.get_all_files();
        files
            .into_iter()
            .filter_map(|file| {
                read_json_file(
                    &self
                        .get_model_path(self.current_model_name.as_ref().unwrap())
                        .join(file),
                )
            })
            .collect()
    }

    pub fn find(&self, condition: &Value) -> Vec<T> {
        self.find_all()
            .into_iter()
            .filter(|item| self.matches_condition(&serde_json::to_value(item).unwrap(), condition))
            .collect()
    }

    pub fn find_one(&self, condition: &Value) -> Option<T> {
        self.find_all()
            .into_iter()
            .find(|item| self.matches_condition(&serde_json::to_value(item).unwrap(), condition))
    }

    pub fn count(&self, condition: &Value) -> usize {
        self.find(condition).len()
    }

    pub fn update_many(&self, condition: &Value, data: &Value) {
        let items = self.find(condition);
        for item in items {
            let id = item.get_id();
            self.update_by_id(&id, data.clone());
        }
    }

    pub fn delete_many(&self, condition: &Value) {
        let items = self.find(condition);
        for item in items {
            let id = item.get_id();
            self.delete_by_id(&id);
        }
    }

    pub fn push(&self, condition: &Value, array_path: &str, element: &Value) {
        let items = self.find(condition);
        for item in items {
            let id = item.get_id();
            if let Some(data) = self.find_by_id(&id) {
                let mut data_json = serde_json::to_value(&data).unwrap();
                let keys: Vec<&str> = array_path.split('.').collect();
                let array = self.get_nested_property(&data_json, &keys);
                if array.is_array() {
                    let mut array = array.as_array().unwrap().clone();
                    array.push(element.clone());
                    self.set_nested_property(&mut data_json, &keys, Value::Array(array));
                    let updated_data: T = serde_json::from_value(data_json).unwrap();
                    self.update_by_id(&id, serde_json::to_value(updated_data).unwrap());
                }
            }
        }
    }

    pub fn pull(&self, condition: &Value, array_path: &str, pull_condition: &Value) {
        let items = self.find(condition);
        for item in items {
            let id = item.get_id();
            if let Some(data) = self.find_by_id(&id) {
                let mut data_json = serde_json::to_value(&data).unwrap();
                let keys: Vec<&str> = array_path.split('.').collect();
                let array = self.get_nested_property(&data_json, &keys);
                if array.is_array() {
                    let new_array: Vec<Value> = array
                        .as_array()
                        .unwrap()
                        .iter()
                        .cloned()
                        .filter(|elem| !self.matches_condition(elem, pull_condition))
                        .collect();
                    self.set_nested_property(&mut data_json, &keys, Value::Array(new_array));
                    let updated_data: T = serde_json::from_value(data_json).unwrap();
                    self.update_by_id(&id, serde_json::to_value(updated_data).unwrap());
                }
            }
        }
    }

    pub fn update_array(
        &self,
        condition: &Value,
        array_path: &str,
        array_condition: &Value,
        updates: &Value,
    ) {
        let items = self.find(condition);
        for item in items {
            let id = item.get_id();
            if let Some(data) = self.find_by_id(&id) {
                let mut data_json = serde_json::to_value(&data).unwrap();
                let keys: Vec<&str> = array_path.split('.').collect();
                let array = self.get_nested_property(&data_json, &keys);
                if array.is_array() {
                    let new_array: Vec<Value> = array
                        .as_array()
                        .unwrap()
                        .iter()
                        .cloned()
                        .map(|elem| {
                            if self.matches_condition(&elem, array_condition) {
                                let mut updated_elem = elem.clone();
                                self.update_nested_object(&mut updated_elem, updates);
                                updated_elem
                            } else {
                                elem
                            }
                        })
                        .collect();
                    self.set_nested_property(&mut data_json, &keys, Value::Array(new_array));
                    let updated_data: T = serde_json::from_value(data_json).unwrap();
                    self.update_by_id(&id, serde_json::to_value(updated_data).unwrap());
                }
            }
        }
    }
}

fn create_directory_if_not_exists(path: &Path) {
    if !path.exists() {
        fs::create_dir_all(path).expect("Unable to create directory");
    }
}

fn read_json_file<T>(path: &Path) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    let mut file = File::open(path).ok()?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;
    from_str(&contents).ok()
}

fn write_json_file<T>(path: &Path, data: &T)
where
    T: Serialize,
{
    let mut file = File::create(path).expect("Unable to create file");
    let contents = to_string_pretty(data).expect("Unable to serialize data");
    file.write_all(contents.as_bytes())
        .expect("Unable to write to file");
}

pub trait Identifiable {
    fn get_id(&self) -> String;
}
