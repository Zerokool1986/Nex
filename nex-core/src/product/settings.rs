use crate::runtime::experience::InterfaceComplexity;

#[derive(Debug, Clone)]
pub struct SettingsConsequenceTree {
    pub active_complexity_slider: InterfaceComplexity,
    pub user_section: Vec<SettingItem>,
    pub your_nex_section: Vec<SettingItem>,
    pub applications_section: Vec<SettingItem>,
    pub system_section: Vec<SettingItem>,
    pub advanced_section: Option<Vec<SettingItem>>,
}

#[derive(Debug, Clone)]
pub struct SettingItem {
    pub key: String,
    pub title: String,
    pub current_value: String,
    pub explanation: String,
}

pub struct SettingsController;

impl SettingsController {
    pub fn build_settings_tree(complexity: InterfaceComplexity) -> SettingsConsequenceTree {
        let user_sec = vec![
            SettingItem {
                key: "profile".to_string(),
                title: "You & Profile".to_string(),
                current_value: "Chris".to_string(),
                explanation: "Your sovereign root profile name".to_string(),
            },
            SettingItem {
                key: "security".to_string(),
                title: "Security & Recovery".to_string(),
                current_value: "3-of-5 Shamir Social Recovery Active".to_string(),
                explanation: "Key guardianship and emergency restore settings".to_string(),
            },
        ];

        let your_nex_sec = vec![
            SettingItem {
                key: "spaces".to_string(),
                title: "Spaces".to_string(),
                current_value: "5 active (Personal, Family, Work, Community, Project)".to_string(),
                explanation: "Human context boundaries".to_string(),
            },
            SettingItem {
                key: "devices".to_string(),
                title: "Devices".to_string(),
                current_value: "3 paired hardware devices".to_string(),
                explanation: "KeyStore hardware-backed hosts under your identity".to_string(),
            },
            SettingItem {
                key: "sync".to_string(),
                title: "Synchronization".to_string(),
                current_value: "Local Mesh + Opaque Store-and-Forward Relay".to_string(),
                explanation: "Anti-entropy replication rules".to_string(),
            },
        ];

        let apps_sec = vec![
            SettingItem {
                key: "photos".to_string(),
                title: "Photos Lens".to_string(),
                current_value: "Auto-organize by Space".to_string(),
                explanation: "Display rules for sovereign media".to_string(),
            },
            SettingItem {
                key: "drive".to_string(),
                title: "Drive Lens".to_string(),
                current_value: "FastCDC Deduplication Active".to_string(),
                explanation: "Content-addressed storage and folder view".to_string(),
            },
        ];

        let system_sec = vec![
            SettingItem {
                key: "notifications".to_string(),
                title: "Activity & Notifications".to_string(),
                current_value: "Calm / Low Noise".to_string(),
                explanation: "Alerts for shared items and device connections".to_string(),
            },
        ];

        let adv_sec = if matches!(complexity, InterfaceComplexity::Advanced | InterfaceComplexity::Expert) {
            Some(vec![
                SettingItem {
                    key: "smt".to_string(),
                    title: "SMT State Root Inspector".to_string(),
                    current_value: "Verified 256-bit Sparse Merkle Tree".to_string(),
                    explanation: "Cryptographic state proofs and frontier logs".to_string(),
                },
                SettingItem {
                    key: "wal".to_string(),
                    title: "Write-Ahead Log".to_string(),
                    current_value: "NEX/WAL/v1 Append-Only Store".to_string(),
                    explanation: "Crash resilience and mutation journal".to_string(),
                },
            ])
        } else {
            None
        };

        SettingsConsequenceTree {
            active_complexity_slider: complexity,
            user_section: user_sec,
            your_nex_section: your_nex_sec,
            applications_section: apps_sec,
            system_section: system_sec,
            advanced_section: adv_sec,
        }
    }
}
