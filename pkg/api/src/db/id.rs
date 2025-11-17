use std::fmt;
use uuid::Uuid;

/// Trait for all ID types in the database.
/// Provides uniform access to the underlying UUID and enables writing functions
/// that accept any kind of ID.
pub trait Id: Copy + fmt::Debug + fmt::Display {
    /// Get the underlying UUID.
    fn as_uuid(&self) -> Uuid;

    /// Create an ID from a UUID.
    fn from_uuid(uuid: Uuid) -> Self;

    /// Generate a new ID (defaults to UUIDv7).
    fn generate() -> Self {
        Self::from_uuid(Uuid::now_v7())
    }
}

/// Macro to define a type-safe ID newtype wrapper around UUID.
///
/// This generates:
/// - A newtype struct wrapping uuid::Uuid
/// - Implementations of: Debug, Clone, Copy, PartialEq, Eq, Display, Id trait
/// - sqlx::Type and sqlx::Encode/Decode for database support
/// - serde Serialize and Deserialize
///
/// # Example
/// ```
/// use api::db::id::Id;
/// use api::define_id;
/// use uuid::Uuid;
///
/// define_id!(FooId);
/// define_id!(BarId);
///
/// fn process_foo(id: FooId) {
///     println!("Processing foo: {}", id);
/// }
///
/// // This prevents mixing up IDs:
/// let foo_id = FooId::generate();
/// let bar_id = BarId::generate();
/// process_foo(foo_id);  // OK
/// // process_foo(bar_id);  // Compile error!
///
/// // Generic functions work with any ID:
/// fn log_any_id(id: impl Id) {
///     println!("ID: {} (UUID: {})", id, id.as_uuid());
/// }
/// log_any_id(foo_id);
/// log_any_id(bar_id);
/// ```
#[macro_export]
macro_rules! define_id {
    ($name:ident) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            serde::Serialize,
            serde::Deserialize,
            sqlx::Type,
        )]
        #[sqlx(transparent)]
        pub struct $name(pub uuid::Uuid);

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl $crate::db::id::Id for $name {
            fn as_uuid(&self) -> uuid::Uuid {
                self.0
            }

            fn from_uuid(uuid: uuid::Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<uuid::Uuid> for $name {
            fn from(uuid: uuid::Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for uuid::Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    define_id!(TestFooId);
    define_id!(TestBarId);

    #[test]
    fn test_id_creation() {
        let uuid = Uuid::now_v7();
        let foo_id = TestFooId::from_uuid(uuid);
        assert_eq!(foo_id.as_uuid(), uuid);
    }


    #[test]
    fn test_id_generate() {
        let id1 = TestFooId::generate();
        let id2 = TestFooId::generate();
        // Generated IDs should always be unique
        assert_ne!(id1, id2);
        assert_eq!(id1.as_uuid().get_version(), Some(uuid::Version::SortRand));
        assert_eq!(id2.as_uuid().get_version(), Some(uuid::Version::SortRand));

        // Should work with any ID type via the trait
        let bar_id = TestBarId::generate();
        assert_ne!(id1.as_uuid(), bar_id.as_uuid());
    }

    #[test]
    fn test_id_display() {
        let uuid = Uuid::now_v7();
        let foo_id = TestFooId::from_uuid(uuid);
        assert_eq!(foo_id.to_string(), uuid.to_string());
    }

    #[test]
    fn test_id_type_safety() {
        let uuid = Uuid::now_v7();
        let foo_id = TestFooId::from_uuid(uuid);
        let bar_id = TestBarId::from_uuid(uuid);

        // Same UUID but different types
        assert_eq!(foo_id.as_uuid(), bar_id.as_uuid());
        // This would be a compile error:
        // let _: TestFooId = bar_id;
    }

    #[test]
    fn test_id_generic_function() {
        fn extract_uuid(id: impl Id) -> Uuid {
            id.as_uuid()
        }

        let uuid = Uuid::now_v7();
        let foo_id = TestFooId::from_uuid(uuid);
        let bar_id = TestBarId::from_uuid(uuid);

        assert_eq!(extract_uuid(foo_id), uuid);
        assert_eq!(extract_uuid(bar_id), uuid);
    }

    #[test]
    fn test_id_conversion() {
        let uuid = Uuid::now_v7();
        let foo_id: TestFooId = uuid.into();
        let back: Uuid = foo_id.into();
        assert_eq!(uuid, back);
    }

    #[test]
    fn test_id_serialization() {
        let uuid = Uuid::now_v7();
        let foo_id = TestFooId::from_uuid(uuid);

        let json = serde_json::to_string(&foo_id).unwrap();
        let deserialized: TestFooId = serde_json::from_str(&json).unwrap();
        assert_eq!(foo_id, deserialized);
    }
}
