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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Product {
    id: String,
    name: String,
    price: f64,
}

impl Identifiable for Product {
    fn get_id(&self) -> String {
        self.id.clone()
    }
}

fn main() {
    // Set the base directory for the database
    JsonDatabase::set_base_dir("custom-dir");

    // Initialize the database with a model
    let mut user_db = JsonDatabase::<User>::new(Some("users"));
    let mut product_db = JsonDatabase::<Product>::new(Some("products"));

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
    user_db.create(&user1.id, user1.clone());
    user_db.create(&user2.id, user2.clone());

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
    user_db.delete_by_id("2");

    // Find all users after deletion
    let all_users_after_deletion = user_db.find_all();
    println!("All users after deletion: {:?}", all_users_after_deletion);

    // Change model to "products"
    product_db.model("products");

    // Example product data
    let product1 = Product {
        id: "1".to_string(),
        name: "Laptop".to_string(),
        price: 999.99,
    };

    let product2 = Product {
        id: "2".to_string(),
        name: "Smartphone".to_string(),
        price: 499.99,
    };

    // Create products
    product_db.create(&product1.id, product1.clone());
    product_db.create(&product2.id, product2.clone());

    // Find all products
    let all_products = product_db.find_all();
    println!("All products: {:?}", all_products);
}
