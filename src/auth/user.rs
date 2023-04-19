use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use nanoid::nanoid;
use rand::Rng;

use crate::util::constant_time_compare;

pub struct User {
    pub id: String,
    pub name: String,
    pub avatar: String,
    pub email: String,
    pub password: String, // todo: remove me! replace with db lookup!
}

pub struct UserManager {
    users: RwLock<HashMap<String, Arc<User>>>,
}

// need to find a way to generate a unique discriminator...
fn generate_discrim() -> String {
    let mut rng = rand::thread_rng();

    String::from_iter([
        rng.gen_range(0..10).to_string(),
        rng.gen_range(0..10).to_string(),
        rng.gen_range(0..10).to_string(),
        rng.gen_range(0..10).to_string(),
    ])
}

pub enum AuthResult {
    Failure,
    Success(Arc<User>),
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            users: RwLock::new(HashMap::new()),
        }
    }

    // todo: implement db stuff
    pub fn register_user(&self, name: &str, email: &str, password: &str) -> Option<Arc<User>> {
        if self
            .users
            .read()
            .unwrap()
            .values()
            .any(|u| u.email == email || u.name == name)
        {
            return None;
        }

        // todo: validate some regex here...
        let user = Arc::new(User {
            id: nanoid!(),
            name: String::from_iter([name, "#", &generate_discrim()]),
            email: email.to_owned(),
            avatar: "https://placehold.co/32".to_owned(),
            password: password.to_owned(),
        });

        self.users
            .write()
            .unwrap()
            .insert(user.id.clone(), user.clone());

        Some(user)
    }

    pub fn get_user_by_id(&self, id: &str) -> Option<Arc<User>> {
        self.users.read().unwrap().get(id).cloned()
    }

    pub fn get_user_by_name(&self, username: &str) -> Option<Arc<User>> {
        self.users
            .read()
            .unwrap()
            .values()
            .find(|user| user.name == username)
            .cloned()
    }

    pub fn get_user_by_email(&self, email: &str) -> Option<Arc<User>> {
        self.users
            .read()
            .unwrap()
            .values()
            .find(|user| user.email == email)
            .cloned()
    }

    pub fn authenticate(&self, email: &str, password: &str) -> AuthResult {
        if let Some(user) = self.get_user_by_email(email) {
            // todo: replace with db lookup!
            if constant_time_compare(&user.password, password) {
                return AuthResult::Success(user);
            }
        }
        AuthResult::Failure
    }
}
