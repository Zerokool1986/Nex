use std::collections::BTreeMap;
use crate::object::types::{ObjectID, NamespaceID, NexObject, ObjectType};
use crate::identity::types::ActorID;

#[derive(Debug, Default, Clone)]
pub struct NexObjectStore {
    pub objects: BTreeMap<ObjectID, NexObject>,
    pub namespace_index: BTreeMap<NamespaceID, Vec<ObjectID>>,
    pub owner_index: BTreeMap<ActorID, Vec<ObjectID>>,
}

impl NexObjectStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, object: NexObject) {
        let obj_id = object.object_id;
        let ns = object.namespace;
        let owner = object.owner_actor_id;

        let should_insert = match self.objects.get(&obj_id) {
            Some(existing) => {
                if matches!(existing.object_type, ObjectType::Synthetic(_)) && !matches!(object.object_type, ObjectType::Synthetic(_)) {
                    true
                } else if !matches!(existing.object_type, ObjectType::Synthetic(_)) && matches!(object.object_type, ObjectType::Synthetic(_)) {
                    object.created_epoch > existing.created_epoch ||
                    (object.created_epoch == existing.created_epoch && object.created_lamport > existing.created_lamport)
                } else {
                    object.created_epoch > existing.created_epoch ||
                    (object.created_epoch == existing.created_epoch && object.created_lamport >= existing.created_lamport)
                }
            }
            None => true,
        };

        if should_insert {
            let ns_entry = self.namespace_index.entry(ns).or_default();
            if !ns_entry.contains(&obj_id) {
                ns_entry.push(obj_id);
            }
            let owner_entry = self.owner_index.entry(owner).or_default();
            if !owner_entry.contains(&obj_id) {
                owner_entry.push(obj_id);
            }
            self.objects.insert(obj_id, object);
        }
    }

    pub fn get(&self, object_id: &ObjectID) -> Option<&NexObject> {
        self.objects.get(object_id)
    }

    pub fn get_mut(&mut self, object_id: &ObjectID) -> Option<&mut NexObject> {
        self.objects.get_mut(object_id)
    }

    pub fn list_by_namespace(&self, namespace: &NamespaceID) -> Vec<&NexObject> {
        if let Some(ids) = self.namespace_index.get(namespace) {
            ids.iter().filter_map(|id| self.objects.get(id)).collect()
        } else {
            Vec::new()
        }
    }

    pub fn tombstone(&mut self, object_id: &ObjectID) -> bool {
        if let Some(obj) = self.objects.get_mut(object_id) {
            obj.tombstoned = true;
            true
        } else {
            false
        }
    }
}
