use std::collections::{BTreeMap, BTreeSet};
use crate::runtime::node::NexNode;
use crate::identity::types::{ActorID, CapabilityProof, OP_WRITE};
use crate::identity::verifier::verify_capability_chain;
use crate::object::types::{NamespaceID, ObjectID, ObjectType};
use crate::api::NexAppApi;

pub struct UiActionDispatcher;

impl UiActionDispatcher {
    pub fn dispatch_ui_create_object(
        node: &mut NexNode,
        proof: &CapabilityProof,
        namespace: NamespaceID,
        object_type: ObjectType,
        metadata: BTreeMap<String, String>,
        payload: Vec<u8>,
        current_epoch: u64,
        revoked_tokens: &BTreeMap<[u8; 32], u64>,
        root_actor: &ActorID,
    ) -> Result<ObjectID, String> {
        // Zero-Ambient: verify capability token proof before allowing UI creation
        verify_capability_chain(
            proof,
            OP_WRITE,
            &namespace,
            None,
            current_epoch,
            revoked_tokens,
            root_actor,
        ).map_err(|e| format!("UiActionDenied: {:?}", e))?;

        node.create_object(namespace, object_type, metadata, payload)
            .map_err(|e| format!("NodeCreateFailed: {:?}", e))
    }

    pub fn dispatch_ui_revoke_device(
        crl: &mut BTreeSet<ActorID>,
        device_actor: ActorID,
    ) -> usize {
        crl.insert(device_actor);
        crl.len()
    }
}
