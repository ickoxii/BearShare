// Database operations for room management

use anyhow::{Context, Result};
use sqlx::{AnyPool, Row};
use uuid::Uuid;

// Database manager for room metadata
#[derive(Clone)]
pub struct Database {
    pool: AnyPool,
}

impl Database {
    // Create a new database connection
    pub async fn new(database_url: &str) -> Result<Self> {
        sqlx::any::install_default_drivers();

        let pool = AnyPool::connect(database_url)
            .await
            .context("Failed to connect to database")?;

        let db = Database { pool };
        db.init().await?;

        Ok(db)
    }

    // Initialize database schema
    async fn init(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS rooms (
                id CHAR(36) PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                password_hash VARCHAR(255) NOT NULL,
                filename VARCHAR(255) NOT NULL,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL,
                active_users INTEGER DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create rooms table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id CHAR(36) PRIMARY KEY,
                room_id CHAR(36) NOT NULL,
                site_id INTEGER NOT NULL,
                connected_at DATETIME NOT NULL,
                FOREIGN KEY (room_id) REFERENCES rooms(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create users table")?;

        tracing::info!("Database initialized successfully");
        Ok(())
    }

    // Create a new room entry
    pub async fn create_room(
        &self,
        id: &str,
        name: &str,
        password_hash: &str,
        filename: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO rooms (id, name, password_hash, filename, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(password_hash)
        .bind(filename)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .context("Failed to create room")?;

        tracing::info!("Created room {} in database", id);
        Ok(())
    }

    // Get room by ID
    pub async fn get_room(&self, room_id: &str) -> Result<Option<RoomRecord>> {
        let result = sqlx::query_as::<_, RoomRecord>(
            r#"
            SELECT id, name, password_hash, filename, created_at, updated_at, active_users
            FROM rooms
            WHERE id = ?
            "#,
        )
        .bind(room_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get room")?;

        Ok(result)
    }

    // Check if room exists
    pub async fn room_exists(&self, room_id: &str) -> Result<bool> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM rooms WHERE id = ?
            "#,
        )
        .bind(room_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to check room existence")?;

        Ok(result.0 > 0)
    }

    // Delete room
    pub async fn delete_room(&self, room_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM rooms WHERE id = ?")
            .bind(room_id)
            .execute(&self.pool)
            .await
            .context("Failed to delete room")?;

        tracing::info!("Deleted room {} from database", room_id);
        Ok(())
    }

    // Add user to room
    pub async fn add_user(&self, user_id: &str, room_id: &str, site_id: u32) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        // MySQL: Use REPLACE INTO to handle reconnections gracefully
        // Note: REPLACE INTO deletes the old row and inserts a new one
        sqlx::query(
            r#"
            REPLACE INTO users (id, room_id, site_id, connected_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(user_id)
        .bind(room_id)
        .bind(site_id as i64)
        .bind(&now)
        .execute(&self.pool)
        .await
        .context("Failed to add user")?;

        // Increment active users count
        sqlx::query("UPDATE rooms SET active_users = active_users + 1 WHERE id = ?")
            .bind(room_id)
            .execute(&self.pool)
            .await
            .context("Failed to update active users")?;

        Ok(())
    }

    // Remove user from room
    pub async fn remove_user(&self, user_id: &str, room_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .context("Failed to remove user")?;

        // Decrement active users count
        sqlx::query("UPDATE rooms SET active_users = GREATEST(0, active_users - 1) WHERE id = ?")
            .bind(room_id)
            .execute(&self.pool)
            .await
            .context("Failed to update active users")?;

        Ok(())
    }

    // Get active user count for room
    pub async fn get_active_users(&self, room_id: &str) -> Result<i64> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT active_users FROM rooms WHERE id = ?
            "#,
        )
        .bind(room_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to get active users")?;

        Ok(result.0)
    }

    // List all rooms
    pub async fn list_rooms(&self) -> Result<Vec<RoomRecord>> {
        let rooms = sqlx::query_as::<_, RoomRecord>(
            r#"
            SELECT id, name, password_hash, filename, created_at, updated_at, active_users
            FROM rooms
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list rooms")?;

        Ok(rooms)
    }

    // Update room's updated_at timestamp
    pub async fn touch_room(&self, room_id: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query("UPDATE rooms SET updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(room_id)
            .execute(&self.pool)
            .await
            .context("Failed to touch room")?;

        Ok(())
    }
}

// Room database record
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RoomRecord {
    pub id: String,
    pub name: String,
    pub password_hash: String,
    pub filename: String,
    pub created_at: String,
    pub updated_at: String,
    pub active_users: i64,
}

impl RoomRecord {
    // Parse created_at as DateTime
    pub fn created_at_parsed(&self) -> Result<chrono::DateTime<chrono::Utc>> {
        chrono::DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .context("Failed to parse created_at")
    }

    // Parse updated_at as DateTime
    pub fn updated_at_parsed(&self) -> Result<chrono::DateTime<chrono::Utc>> {
        chrono::DateTime::parse_from_rfc3339(&self.updated_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .context("Failed to parse updated_at")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Requires MySQL database - sqlx 'any' driver doesn't support SQLite DATETIME"]
    async fn test_database_operations() {
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "mysql://root:password@127.0.0.1:3307/bearshare".to_string());

        let db = Database::new(&db_url).await.unwrap();

        let room_id = Uuid::new_v4().to_string();

        // Create room
        db.create_room(&room_id, "Test Room", "hash", "test.txt")
            .await
            .unwrap();

        // Check exists
        assert!(db.room_exists(&room_id).await.unwrap());

        // Get room
        let room = db.get_room(&room_id).await.unwrap();
        assert!(room.is_some());
        assert_eq!(room.unwrap().name, "Test Room");

        // Add user
        let user_id = Uuid::new_v4().to_string();
        db.add_user(&user_id, &room_id, 1).await.unwrap();

        let count = db.get_active_users(&room_id).await.unwrap();
        assert_eq!(count, 1);

        // Remove user
        db.remove_user(&user_id, &room_id).await.unwrap();
        let count = db.get_active_users(&room_id).await.unwrap();
        assert_eq!(count, 0);

        // Delete room
        db.delete_room(&room_id).await.unwrap();
        assert!(!db.room_exists(&room_id).await.unwrap());
    }
}
