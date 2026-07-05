use lv_api_types::users::UserResponse;

use crate::client::ApiClient;

pub async fn run(client: &ApiClient) -> anyhow::Result<()> {
    let user: UserResponse = client.get("/api/v1/users/me").await?;
    println!("id:       {}", user.id);
    println!("username: {}", user.username);
    println!("email:    {}", user.email);
    println!("admin:    {}", user.is_admin);
    Ok(())
}
