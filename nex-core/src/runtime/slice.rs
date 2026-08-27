use std::collections::BTreeMap;
use crate::runtime::node::NexNode;
use crate::runtime::shell::{NexHomeShell, SpaceType};
use crate::runtime::diagnostics::{SubstrateHealthDiagnostics, ProgressiveTier};
use crate::runtime::dispatcher::UiActionDispatcher;
use crate::sync::anti_entropy::AntiEntropyEngine;
use crate::object::types::{ObjectID, ObjectType, NamespaceID};
use crate::identity::types::{ActorID, CapabilityProof};

pub struct SovereignProductSlice;

impl SovereignProductSlice {
    /// 1. Mobile Host captures a photo in Family Space with capability gating
    pub fn mobile_capture_family_photo(
        mobile_node: &mut NexNode,
        proof: &CapabilityProof,
        title: &str,
        photo_bytes: Vec<u8>,
        current_epoch: u64,
        revoked_tokens: &BTreeMap<[u8; 32], u64>,
        root_actor: &ActorID,
    ) -> Result<(ObjectID, NamespaceID), String> {
        let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
        let mut meta = BTreeMap::new();
        meta.insert("title".to_string(), title.to_string());
        meta.insert("space".to_string(), "Family".to_string());

        let obj_id = UiActionDispatcher::dispatch_ui_create_object(
            mobile_node,
            proof,
            family_ns,
            ObjectType::PhotoMedia,
            meta,
            photo_bytes,
            current_epoch,
            revoked_tokens,
            root_actor,
        )?;

        Ok((obj_id, family_ns))
    }

    /// 2. Mobile Host creates a document in Family Space with capability gating
    pub fn mobile_create_family_document(
        mobile_node: &mut NexNode,
        proof: &CapabilityProof,
        filename: &str,
        doc_bytes: Vec<u8>,
        current_epoch: u64,
        revoked_tokens: &BTreeMap<[u8; 32], u64>,
        root_actor: &ActorID,
    ) -> Result<(ObjectID, NamespaceID), String> {
        let family_ns = NexHomeShell::space_to_namespace(SpaceType::Family);
        let mut meta = BTreeMap::new();
        meta.insert("filename".to_string(), filename.to_string());
        meta.insert("space".to_string(), "Family".to_string());

        let obj_id = UiActionDispatcher::dispatch_ui_create_object(
            mobile_node,
            proof,
            family_ns,
            ObjectType::DriveInode,
            meta,
            doc_bytes,
            current_epoch,
            revoked_tokens,
            root_actor,
        )?;

        Ok((obj_id, family_ns))
    }

    /// 3. Synchronize Mobile Node -> Desktop Node via SMT Anti-Entropy
    pub fn sync_mobile_to_desktop(
        mobile_node: &mut NexNode,
        desktop_node: &mut NexNode,
    ) -> usize {
        let session_id = [0x42; 16];
        let adv_desktop = AntiEntropyEngine::generate_advertise(desktop_node, session_id);
        let batches = AntiEntropyEngine::generate_batches_for_peer(
            mobile_node,
            session_id,
            &adv_desktop.frontier_mutation_ids,
            100,
        );

        let mut ingested_count = 0;
        for batch in batches {
            if AntiEntropyEngine::ingest_batch(desktop_node, batch).is_ok() {
                ingested_count += 1;
            }
        }

        // Reconcile object store metadata & namespace
        for (id, obj) in &mobile_node.state.object_store {
            if let Some(target) = desktop_node.state.object_store.get_mut(id) {
                target.namespace = obj.namespace;
                target.object_type = obj.object_type;
                target.metadata = obj.metadata.clone();
                target.tombstoned = obj.tombstoned;
            }
        }
        for (id, obj) in &desktop_node.state.object_store {
            if let Some(target) = mobile_node.state.object_store.get_mut(id) {
                target.namespace = obj.namespace;
                target.object_type = obj.object_type;
                target.metadata = obj.metadata.clone();
                target.tombstoned = obj.tombstoned;
            }
        }

        ingested_count
    }

    /// 4. Desktop Host verifies and presents Family Space items through NEX Control Shell
    pub fn desktop_present_family_space(
        desktop_node: &NexNode,
    ) -> (usize, Vec<String>, String) {
        let mut shell = NexHomeShell::new();
        shell.switch_space(SpaceType::Family);

        let summary = shell.generate_home_summary(desktop_node);
        let activity = shell.recent_activity_for_space(desktop_node, SpaceType::Family);
        let titles: Vec<String> = activity.into_iter().map(|item| item.title).collect();
        let diag = SubstrateHealthDiagnostics::format_sync_state(desktop_node, ProgressiveTier::Everyday);

        (summary.total_objects_in_space, titles, diag)
    }
}
