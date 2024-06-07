use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty, Value};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const BASE_DIR: &str = "json-db";

/// A simple JSON file-based database ORM for Rust.
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
    /// Creates a new `JsonDatabase` instance.
    ///
    /// # Arguments
    ///
    /// * `model_name` - Optional model name to initialize the database with.
    ///
    /// # Examples
    ///
    /// ```
    /// let db: JsonDatabase<User> = JsonDatabase::new(Some("users"));
    /// ```
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

    /// Sets the model name for the database and ensures the directory exists.
    ///
    /// # Arguments
    ///
    /// * `model_name` - The name of the model to use.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut db: JsonDatabase<User> = JsonDatabase::new(None);
    /// db.model("users");
    /// ```
    pub fn model(&mut self, model_name: &str) -> &mut Self {
        self.current_model_name = Some(model_name.to_string());
        create_directory_if_not_exists(&self.get_model_path(model_name));
        self
    }

    /// Returns the path to the model directory.
    ///
    /// # Arguments
    ///
    /// * `model_name` - The name of the model.
    fn get_model_path(&self, model_name: &str) -> PathBuf {
        Path::new(BASE_DIR).join(model_name)
    }

    /// Returns the path to a specific file in the model directory.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the file.
    fn get_file_path(&self, id: &str) -> PathBuf {
        let model_name = self
            .current_model_name
            .as_ref()
            .expect("Model name is not specified");
        self.get_model_path(model_name).join(format!("{}.json", id))
    }

    /// Returns a list of all JSON files in the model directory.
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

    /// Checks if a JSON object matches a given condition.
    ///
    /// # Arguments
    ///
    /// * `item` - The JSON object to check.
    /// * `condition` - The condition to match against.
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

    /// Gets a nested property from a JSON object.
    ///
    /// # Arguments
    ///
    /// * `obj` - The JSON object.
    /// * `keys` - The keys to the nested property.
    fn get_nested_property<'a>(&self, obj: &'a Value, keys: &[&str]) -> &'a Value {
        keys.iter()
            .fold(obj, |acc, key| acc.get(*key).unwrap_or(&Value::Null))
    }

    /// Sets a nested property in a JSON object.
    ///
    /// # Arguments
    ///
    /// * `obj` - The JSON object.
    /// * `keys` - The keys to the nested property.
    /// * `value` - The value to set.
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

    /// Updates a nested property in a JSON object.
    ///
    /// # Arguments
    ///
    /// * `target` - The JSON object to update.
    /// * `source` - The source JSON object containing updates.
    fn update_nested_object(&self, target: &mut Value, source: &Value) {
        for (key, value) in source.as_object().unwrap().iter() {
            let keys: Vec<&str> = key.split('.').collect();
            self.set_nested_property(target, &keys, value.clone());
        }
    }

    /// Creates a new model in the database.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to create.
    ///
    /// # Panics
    ///
    /// Panics if the data does not have an ID field.
    pub fn create_model(&self, data: T) {
        let id = data.get_id();
        if id.is_empty() {
            panic!("Data must have an id field");
        }
        self.create(&id, data);
    }

    /// Creates a new record in the database.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the record.
    /// * `data` - The data to create.
    pub fn create(&self, id: &str, data: T) {
        let file_path = self.get_file_path(id);
        write_json_file(&file_path, &data);
    }

    /// Finds a record by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the record to find.
    ///
    /// # Returns
    ///
    /// Returns `Some(T)` if the record is found, or `None` if not.
    pub fn find_by_id(&self, id: &str) -> Option<T> {
        let file_path = self.get_file_path(id);
        read_json_file(&file_path)
    }

    /// Updates a record by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the record to update.
    /// * `data` - The data to update.
    pub fn update_by_id(&self, id: &str, data: Value) {
        let file_path = self.get_file_path(id);

        if let Some(mut existing_data) = self.find_by_id(id) {
            let mut existing_json = serde_json::to_value(&existing_data).unwrap();

            self.update_nested_object(&mut existing_json, &data);
            let updated_data: T = serde_json::from_value(existing_json).unwrap();
            write_json_file(&file_path, &updated_data);
        }
    }

    /// Deletes a record by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the record to delete.
    pub fn delete_by_id(&self, id: &str) {
        let file_path = self.get_file_path(id);
        if file_path.exists() {
            fs::remove_file(file_path).expect("Unable to delete file");
        }
    }

    /// Finds all records.
    ///
    /// # Returns
    ///
    /// Returns a vector of all records.
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

    /// Finds records matching a condition.
    ///
    /// # Arguments
    ///
    /// * `condition` - The condition to match.
    ///
    /// # Returns
    ///
    /// Returns a vector of matching records.
    pub fn find(&self, condition: &Value) -> Vec<T> {
        self.find_all()
            .into_iter()
            .filter(|item| self.matches_condition(&serde_json::to_value(item).unwrap(), condition))
            .collect()
    }

    /// Finds the first record matching a condition.
    ///
    /// # Arguments
    ///
    /// * `condition` - The condition to match.
    ///
    /// # Returns
    ///
    /// Returns `Some(T)` if a matching record is found, or `None` if not.
    pub fn find_one(&self, condition: &Value) -> Option<T> {
        self.find_all()
            .into_iter()
            .find(|item| self.matches_condition(&serde_json::to_value(item).unwrap(), condition))
    }

    /// Counts the number of records matching a condition.
    ///
    /// # Arguments
    ///
    /// * `condition` - The condition to match.
    ///
    /// # Returns
    ///
    /// Returns the number of matching records.
    pub fn count(&self, condition: &Value) -> usize {
        self.find(condition).len()
    }

    /// Updates multiple records matching a condition.
    ///
    /// # Arguments
    ///
    /// * `condition` - The condition to match.
    /// * `data` - The data to update.
    pub fn update_many(&self, condition: &Value, data: &Value) {
        let items = self.find(condition);
        for item in items {
            let id = item.get_id();
            self.update_by_id(&id, data.clone());
        }
    }

    /// Deletes multiple records matching a condition.
    ///
    /// # Arguments
    ///
    /// * `condition` - The condition to match.
    pub fn delete_many(&self, condition: &Value) {
        let items = self.find(condition);
        for item in items {
            let id = item.get_id();
            self.delete_by_id(&id);
        }
    }

    /// Adds an element to an array in records matching a condition.
    ///
    /// # Arguments
    ///
    /// * `condition` - The condition to match.
    /// * `array_path` - The path to the array.
    /// * `element` - The element to add.
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

    /// Removes elements from an array in records matching a condition.
    ///
    /// # Arguments
    ///
    /// * `condition` - The condition to match.
    /// * `array_path` - The path to the array.
    /// * `pull_condition` - The condition to match elements to remove.
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

    /// Updates elements in an array in records matching a condition.
    ///
    /// # Arguments
    ///
    /// * `condition` - The condition to match.
    /// * `array_path` - The path to the array.
    /// * `array_condition` - The condition to match array elements.
    /// * `updates` - The updates to apply to matching elements.
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

/// Creates a directory if it does not exist.
///
/// # Arguments
///
/// * `path` - The path of the directory to create.
fn create_directory_if_not_exists(path: &Path) {
    if !path.exists() {
        fs::create_dir_all(path).expect("Unable to create directory");
    }
}

/// Reads a JSON file and deserializes it into a Rust struct.
///
/// # Arguments
///
/// * `path` - The path of the JSON file to read.
///
/// # Returns
///
/// Returns `Some(T)` if successful, or `None` if there is an error.
fn read_json_file<T>(path: &Path) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    let mut file = File::open(path).ok()?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;
    from_str(&contents).ok()
}

/// Serializes a Rust struct into JSON and writes it to a file.
///
/// # Arguments
///
/// * `path` - The path of the file to write.
/// * `data` - The data to serialize and write.
fn write_json_file<T>(path: &Path, data: &T)
where
    T: Serialize,
{
    let mut file = File::create(path).expect("Unable to create file");
    let contents = to_string_pretty(data).expect("Unable to serialize data");
    file.write_all(contents.as_bytes())
        .expect("Unable to write to file");
}

/// A trait for types that have an ID.
pub trait Identifiable {
    /// Returns the ID of the object.
    fn get_id(&self) -> String;
}
