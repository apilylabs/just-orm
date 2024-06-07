use just_orm::{Identifiable, JsonDatabase};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: String,
    name: String,
    email: String,
}

impl Identifiable for User {
    fn get_id(&self) -> String {
        self.id.clone()
    }
}

fn main() {
    // Initialize the database with a custom base directory
    let mut user_db: JsonDatabase<User> = JsonDatabase::new("custom-dir", Some("users"));

    // Example user data
    let user1 = User {
        id: "1".to_string(),
        name: "John Doe".to_string(),
        email: "john.doe@example.com".to_string(),
    };

    let user2 = User {
        id: "2".to_string(),
        name: "Jane Smith".to_string(),
        email: "jane.smith@example.com".to_string(),
    };

    // Create users
    user_db.create_model(user1);
    user_db.create_model(user2);

    // Find a user by ID
    if let Some(user) = user_db.find_by_id("1") {
        println!("Found user: {:?}", user);
    } else {
        println!("User not found");
    }

    // Update a user's information
    let update_data = json!({
        "name": "Johnathan Doe"
    });
    user_db.update_by_id("1", update_data);

    // Find all users
    let all_users = user_db.find_all();
    println!("All users: {:?}", all_users);

    // Find users by condition
    let condition = json!({
        "email": "jane.smith@example.com"
    });
    let found_users = user_db.find(&condition);
    println!("Found users: {:?}", found_users);

    // Delete a user by ID
    // user_db.delete_by_id("2");

    // Find all users after deletion
    let all_users_after_deletion = user_db.find_all();
    println!("All users after deletion: {:?}", all_users_after_deletion);
}
