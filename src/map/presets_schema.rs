use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Presets {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub chunk: Vec<Chunk>,
    pub group: Vec<PresetsGroup>,
}

#[derive(Serialize, Deserialize)]
pub struct Chunk {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "$text")]
    pub text_content: Option<String>,
    pub combo: Option<Vec<PresetsChunkCombo>>,
    pub reference: Option<Vec<PresetsChunkReference>>,
    pub space: Option<Vec<PresetsChunkSpace>>,
    pub text: Option<Vec<PresetsChunkText>>,
    pub checkgroup: Option<PresetsChunkCheckgroup>,
    pub optional: Option<PresetsChunkOptional>,
    pub check: Option<Vec<PresetsChunkCheck>>,
    pub multiselect: Option<PresetsChunkMultiselect>,
    pub label: Option<PresetsChunkLabel>,
    pub list_entry: Option<Vec<PresetsChunkListEntry>>,
    pub key: Option<PresetsChunkKey>,
    pub roles: Option<PresetsChunkRoles>,
    pub role: Option<Vec<PresetsChunkRole>>,
    pub preset_link: Option<Vec<PresetsChunkPresetLink>>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkCombo {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@values")]
    pub values: Option<String>,
    #[serde(rename = "@values_sort")]
    pub values_sort: Option<String>,
    #[serde(rename = "@values_searchable")]
    pub values_searchable: Option<String>,
    #[serde(rename = "@length")]
    pub length: Option<String>,
    #[serde(rename = "@values_context")]
    pub values_context: Option<String>,
    #[serde(rename = "@values_no_i18n")]
    pub values_no_i18n: Option<String>,
    #[serde(rename = "@delimiter")]
    pub delimiter: Option<String>,
    #[serde(rename = "@default")]
    pub default: Option<String>,
    #[serde(rename = "@match")]
    pub combo_match: Option<String>,
    #[serde(rename = "@values_i18n")]
    pub values_i18n: Option<String>,
    #[serde(rename = "$text")]
    pub text_content: Option<String>,
    pub list_entry: Option<Vec<PresetsChunkComboListEntry>>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkComboListEntry {
    #[serde(rename = "@value")]
    pub value: String,
    #[serde(rename = "@short_description")]
    pub short_description: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkReference {
    #[serde(rename = "@ref")]
    pub reference_ref: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkSpace {
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkText {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@match")]
    pub text_match: Option<String>,
    #[serde(rename = "@length")]
    pub length: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkCheckgroup {
    #[serde(rename = "@columns")]
    pub columns: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub check: Vec<PresetsChunkCheckgroupCheck>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkCheckgroupCheck {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@disable_off")]
    pub disable_off: Option<String>,
    #[serde(rename = "@icon")]
    pub icon: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkOptional {
    #[serde(rename = "@text")]
    pub text_attr: Option<String>,
    #[serde(rename = "$text")]
    pub text_content: Option<String>,
    pub checkgroup: Option<PresetsChunkOptionalCheckgroup>,
    pub space: Option<Vec<PresetsChunkOptionalSpace>>,
    pub reference: Option<PresetsChunkOptionalReference>,
    pub combo: Option<Vec<PresetsChunkOptionalCombo>>,
    pub text: Option<Vec<PresetsChunkOptionalText>>,
    pub check: Option<Vec<PresetsChunkOptionalCheck>>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkOptionalCheckgroup {
    #[serde(rename = "@columns")]
    pub columns: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub check: Vec<PresetsChunkOptionalCheckgroupCheck>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkOptionalCheckgroupCheck {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@icon")]
    pub icon: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkOptionalSpace {
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkOptionalReference {
    #[serde(rename = "@ref")]
    pub reference_ref: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkOptionalCombo {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@values")]
    pub values: String,
    #[serde(rename = "@values_sort")]
    pub values_sort: Option<String>,
    #[serde(rename = "@values_context")]
    pub values_context: Option<String>,
    #[serde(rename = "@length")]
    pub length: Option<String>,
    #[serde(rename = "@default")]
    pub default: Option<String>,
    #[serde(rename = "@values_searchable")]
    pub values_searchable: Option<String>,
    #[serde(rename = "@values_no_i18n")]
    pub values_no_i18n: Option<String>,
    #[serde(rename = "@delimiter")]
    pub delimiter: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkOptionalText {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@length")]
    pub length: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkOptionalCheck {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkCheck {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@disable_off")]
    pub disable_off: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkMultiselect {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@values")]
    pub values: Option<String>,
    #[serde(rename = "@values_searchable")]
    pub values_searchable: Option<String>,
    #[serde(rename = "@values_context")]
    pub values_context: Option<String>,
    #[serde(rename = "@values_no_i18n")]
    pub values_no_i18n: Option<String>,
    #[serde(rename = "@values_sort")]
    pub values_sort: Option<String>,
    #[serde(rename = "$text")]
    pub text_content: Option<String>,
    pub list_entry: Option<Vec<PresetsChunkMultiselectListEntry>>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkMultiselectListEntry {
    #[serde(rename = "@value")]
    pub value: String,
    #[serde(rename = "@icon")]
    pub icon: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkLabel {
    #[serde(rename = "@text")]
    pub text: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkListEntry {
    #[serde(rename = "@value")]
    pub value: String,
    #[serde(rename = "@display_value")]
    pub display_value: String,
    #[serde(rename = "@icon")]
    pub icon: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkKey {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@value")]
    pub value: String,
    #[serde(rename = "@match")]
    pub key_match: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkRoles {
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub role: Vec<PresetsChunkRolesRole>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkRolesRole {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@requisite")]
    pub requisite: String,
    #[serde(rename = "@type")]
    pub role_type: String,
    #[serde(rename = "@member_expression")]
    pub member_expression: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkRole {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@requisite")]
    pub requisite: String,
    #[serde(rename = "@type")]
    pub role_type: String,
    #[serde(rename = "@member_expression")]
    pub member_expression: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsChunkPresetLink {
    #[serde(rename = "@preset_name")]
    pub preset_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroup {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@icon")]
    pub icon: String,
    #[serde(rename = "@name_context")]
    pub name_context: Option<String>,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub group: Option<Vec<PresetsGroupGroup>>,
    pub separator: Option<Vec<PresetsGroupSeparator>>,
    pub item: Option<Vec<PresetsGroupItem>>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroup {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@icon")]
    pub icon: String,
    #[serde(rename = "@name_context")]
    pub name_context: Option<String>,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub item: Vec<PresetsGroupGroupItem>,
    pub separator: Option<Vec<PresetsGroupGroupSeparator>>,
    pub group: Option<Vec<PresetsGroupGroupGroup>>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItem {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@icon")]
    pub icon: Option<String>,
    #[serde(rename = "@type")]
    pub item_type: String,
    #[serde(rename = "@preset_name_label")]
    pub preset_name_label: Option<String>,
    #[serde(rename = "@name_context")]
    pub name_context: Option<String>,
    #[serde(rename = "$text")]
    pub text_content: Option<String>,
    pub link: PresetsGroupGroupItemLink,
    pub space: Option<Vec<PresetsGroupGroupItemSpace>>,
    pub key: Option<Vec<PresetsGroupGroupItemKey>>,
    pub text: Option<Vec<PresetsGroupGroupItemText>>,
    pub check: Option<Vec<PresetsGroupGroupItemCheck>>,
    pub optional: Option<Vec<PresetsGroupGroupItemOptional>>,
    pub reference: Option<Vec<PresetsGroupGroupItemReference>>,
    pub combo: Option<Vec<PresetsGroupGroupItemCombo>>,
    pub label: Option<Vec<PresetsGroupGroupItemLabel>>,
    pub checkgroup: Option<Vec<PresetsGroupGroupItemCheckgroup>>,
    pub preset_link: Option<Vec<PresetsGroupGroupItemPresetLink>>,
    pub multiselect: Option<PresetsGroupGroupItemMultiselect>,
    pub roles: Option<PresetsGroupGroupItemRoles>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemLink {
    #[serde(rename = "@wiki")]
    pub wiki: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemSpace {
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemKey {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@value")]
    pub value: String,
    #[serde(rename = "@match")]
    pub key_match: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemText {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@default")]
    pub default: Option<String>,
    #[serde(rename = "@match")]
    pub text_match: Option<String>,
    #[serde(rename = "@length")]
    pub length: Option<String>,
    #[serde(rename = "@delete_if_empty")]
    pub delete_if_empty: Option<String>,
    #[serde(rename = "@multiline")]
    pub multiline: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemCheck {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@disable_off")]
    pub disable_off: Option<String>,
    #[serde(rename = "@value_on")]
    pub value_on: Option<String>,
    #[serde(rename = "@text_context")]
    pub text_context: Option<String>,
    #[serde(rename = "@default")]
    pub default: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemOptional {
    #[serde(rename = "@text")]
    pub text_attr: Option<String>,
    #[serde(rename = "$text")]
    pub text_content: Option<String>,
    pub text: Option<Vec<PresetsGroupGroupItemOptionalText>>,
    pub check: Option<Vec<PresetsGroupGroupItemOptionalCheck>>,
    pub reference: Option<Vec<PresetsGroupGroupItemOptionalReference>>,
    pub space: Option<Vec<PresetsGroupGroupItemOptionalSpace>>,
    pub checkgroup: Option<Vec<PresetsGroupGroupItemOptionalCheckgroup>>,
    pub combo: Option<Vec<PresetsGroupGroupItemOptionalCombo>>,
    pub label: Option<Vec<PresetsGroupGroupItemOptionalLabel>>,
    pub link: Option<PresetsGroupGroupItemOptionalLink>,
    pub multiselect: Option<PresetsGroupGroupItemOptionalMultiselect>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemOptionalText {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@alternative_autocomplete_keys")]
    pub alternative_autocomplete_keys: Option<String>,
    #[serde(rename = "@length")]
    pub length: Option<String>,
    #[serde(rename = "@match")]
    pub text_match: Option<String>,
    #[serde(rename = "@auto_increment")]
    pub auto_increment: Option<String>,
    #[serde(rename = "@use_last_as_default")]
    pub use_last_as_default: Option<String>,
    #[serde(rename = "@multiline")]
    pub multiline: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemOptionalCheck {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@disable_off")]
    pub disable_off: Option<String>,
    #[serde(rename = "@value_on")]
    pub value_on: Option<String>,
    #[serde(rename = "@text_context")]
    pub text_context: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemOptionalReference {
    #[serde(rename = "@ref")]
    pub reference_ref: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemOptionalSpace {
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemOptionalCheckgroup {
    #[serde(rename = "@columns")]
    pub columns: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub check: Vec<PresetsGroupGroupItemOptionalCheckgroupCheck>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemOptionalCheckgroupCheck {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@default")]
    pub default: Option<String>,
    #[serde(rename = "@disable_off")]
    pub disable_off: Option<String>,
    #[serde(rename = "@icon")]
    pub icon: Option<String>,
    #[serde(rename = "@text_context")]
    pub text_context: Option<String>,
    #[serde(rename = "@value_on")]
    pub value_on: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemOptionalCombo {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@values")]
    pub values: Option<String>,
    #[serde(rename = "@values_context")]
    pub values_context: Option<String>,
    #[serde(rename = "@length")]
    pub length: Option<String>,
    #[serde(rename = "@values_sort")]
    pub values_sort: Option<String>,
    #[serde(rename = "@values_no_i18n")]
    pub values_no_i18n: Option<String>,
    #[serde(rename = "@values_searchable")]
    pub values_searchable: Option<String>,
    #[serde(rename = "@match")]
    pub combo_match: Option<String>,
    #[serde(rename = "@values_i18n")]
    pub values_i18n: Option<String>,
    #[serde(rename = "@delimiter")]
    pub delimiter: Option<String>,
    #[serde(rename = "@default")]
    pub default: Option<String>,
    #[serde(rename = "@use_last_as_default")]
    pub use_last_as_default: Option<String>,
    #[serde(rename = "@values_from")]
    pub values_from: Option<String>,
    #[serde(rename = "@text_context")]
    pub text_context: Option<String>,
    #[serde(rename = "@display_values")]
    pub display_values: Option<String>,
    #[serde(rename = "$text")]
    pub text_content: Option<String>,
    pub list_entry: Option<Vec<PresetsGroupGroupItemOptionalComboListEntry>>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemOptionalComboListEntry {
    #[serde(rename = "@value")]
    pub value: String,
    #[serde(rename = "@display_value")]
    pub display_value: Option<String>,
    #[serde(rename = "@short_description")]
    pub short_description: Option<String>,
    #[serde(rename = "@icon")]
    pub icon: Option<String>,
    #[serde(rename = "@icon_size")]
    pub icon_size: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemOptionalLabel {
    #[serde(rename = "@text")]
    pub text: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemOptionalLink {
    #[serde(rename = "@wiki")]
    pub wiki: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemOptionalMultiselect {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@values")]
    pub values: String,
    #[serde(rename = "@rows")]
    pub rows: Option<String>,
    #[serde(rename = "@text_context")]
    pub text_context: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemReference {
    #[serde(rename = "@ref")]
    pub reference_ref: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemCombo {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@values")]
    pub values: Option<String>,
    #[serde(rename = "@values_context")]
    pub values_context: Option<String>,
    #[serde(rename = "@default")]
    pub default: Option<String>,
    #[serde(rename = "@values_sort")]
    pub values_sort: Option<String>,
    #[serde(rename = "@match")]
    pub combo_match: Option<String>,
    #[serde(rename = "@values_searchable")]
    pub values_searchable: Option<String>,
    #[serde(rename = "@values_i18n")]
    pub values_i18n: Option<String>,
    #[serde(rename = "@delimiter")]
    pub delimiter: Option<String>,
    #[serde(rename = "@values_no_i18n")]
    pub values_no_i18n: Option<String>,
    #[serde(rename = "@display_values")]
    pub display_values: Option<String>,
    #[serde(rename = "@text_context")]
    pub text_context: Option<String>,
    #[serde(rename = "$text")]
    pub text_content: Option<String>,
    pub list_entry: Option<Vec<PresetsGroupGroupItemComboListEntry>>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemComboListEntry {
    #[serde(rename = "@value")]
    pub value: String,
    #[serde(rename = "@display_value")]
    pub display_value: Option<String>,
    #[serde(rename = "@icon")]
    pub icon: Option<String>,
    #[serde(rename = "@short_description")]
    pub short_description: Option<String>,
    #[serde(rename = "@regions")]
    pub regions: Option<String>,
    #[serde(rename = "@icon_size")]
    pub icon_size: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemLabel {
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@text_context")]
    pub text_context: Option<String>,
    #[serde(rename = "@icon_size")]
    pub icon_size: Option<String>,
    #[serde(rename = "@icon")]
    pub icon: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemCheckgroup {
    #[serde(rename = "@columns")]
    pub columns: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub check: Vec<PresetsGroupGroupItemCheckgroupCheck>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemCheckgroupCheck {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@disable_off")]
    pub disable_off: Option<String>,
    #[serde(rename = "@delete_if_empty")]
    pub delete_if_empty: Option<String>,
    #[serde(rename = "@default")]
    pub default: Option<String>,
    #[serde(rename = "@text_context")]
    pub text_context: Option<String>,
    #[serde(rename = "@icon")]
    pub icon: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemPresetLink {
    #[serde(rename = "@preset_name")]
    pub preset_name: String,
    #[serde(rename = "@text")]
    pub text: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemMultiselect {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@text_context")]
    pub text_context: Option<String>,
    #[serde(rename = "@values")]
    pub values: String,
    #[serde(rename = "@rows")]
    pub rows: Option<String>,
    #[serde(rename = "@values_no_i18n")]
    pub values_no_i18n: Option<String>,
    #[serde(rename = "@values_sort")]
    pub values_sort: Option<String>,
    #[serde(rename = "@values_searchable")]
    pub values_searchable: Option<String>,
    #[serde(rename = "@values_context")]
    pub values_context: Option<String>,
    #[serde(rename = "@display_values")]
    pub display_values: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemRoles {
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub role: Option<Vec<PresetsGroupGroupItemRolesRole>>,
    pub reference: Option<Vec<PresetsGroupGroupItemRolesReference>>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemRolesRole {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@requisite")]
    pub requisite: String,
    #[serde(rename = "@type")]
    pub role_type: String,
    #[serde(rename = "@member_expression")]
    pub member_expression: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupItemRolesReference {
    #[serde(rename = "@ref")]
    pub reference_ref: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupSeparator {
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupGroup {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@icon")]
    pub icon: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub item: Vec<PresetsGroupGroupGroupItem>,
    pub separator: Option<Vec<PresetsGroupGroupGroupSeparator>>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupGroupItem {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@icon")]
    pub icon: String,
    #[serde(rename = "@type")]
    pub item_type: String,
    #[serde(rename = "@preset_name_label")]
    pub preset_name_label: String,
    #[serde(rename = "@name_context")]
    pub name_context: Option<String>,
    #[serde(rename = "@match_expression")]
    pub match_expression: Option<String>,
    #[serde(rename = "$text")]
    pub text_content: Option<String>,
    pub link: PresetsGroupGroupGroupItemLink,
    pub space: Vec<PresetsGroupGroupGroupItemSpace>,
    pub key: Option<Vec<PresetsGroupGroupGroupItemKey>>,
    pub reference: Option<Vec<PresetsGroupGroupGroupItemReference>>,
    pub text: Option<Vec<PresetsGroupGroupGroupItemText>>,
    pub optional: Option<PresetsGroupGroupGroupItemOptional>,
    pub combo: Option<Vec<PresetsGroupGroupGroupItemCombo>>,
    pub multiselect: Option<PresetsGroupGroupGroupItemMultiselect>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupGroupItemLink {
    #[serde(rename = "@wiki")]
    pub wiki: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupGroupItemSpace {
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupGroupItemKey {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@value")]
    pub value: String,
    #[serde(rename = "@match")]
    pub key_match: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupGroupItemReference {
    #[serde(rename = "@ref")]
    pub reference_ref: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupGroupItemText {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@length")]
    pub length: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupGroupItemOptional {
    #[serde(rename = "$text")]
    pub text_content: Option<String>,
    pub text: PresetsGroupGroupGroupItemOptionalText,
    pub combo: PresetsGroupGroupGroupItemOptionalCombo,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupGroupItemOptionalText {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupGroupItemOptionalCombo {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@values")]
    pub values: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupGroupItemCombo {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@text_context")]
    pub text_context: Option<String>,
    #[serde(rename = "@values")]
    pub values: Option<String>,
    #[serde(rename = "@default")]
    pub default: Option<String>,
    #[serde(rename = "@values_no_i18n")]
    pub values_no_i18n: Option<String>,
    #[serde(rename = "@values_sort")]
    pub values_sort: Option<String>,
    #[serde(rename = "@values_context")]
    pub values_context: Option<String>,
    #[serde(rename = "@values_searchable")]
    pub values_searchable: Option<String>,
    #[serde(rename = "$text")]
    pub text_content: Option<String>,
    pub reference: Option<PresetsGroupGroupGroupItemComboReference>,
    pub list_entry: Option<Vec<PresetsGroupGroupGroupItemComboListEntry>>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupGroupItemComboReference {
    #[serde(rename = "@ref")]
    pub reference_ref: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupGroupItemComboListEntry {
    #[serde(rename = "@value")]
    pub value: String,
    #[serde(rename = "@short_description")]
    pub short_description: Option<String>,
    #[serde(rename = "@display_value")]
    pub display_value: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupGroupItemMultiselect {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@values_searchable")]
    pub values_searchable: String,
    #[serde(rename = "@values_sort")]
    pub values_sort: String,
    #[serde(rename = "$text")]
    pub text_content: Option<String>,
    pub reference: PresetsGroupGroupGroupItemMultiselectReference,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupGroupItemMultiselectReference {
    #[serde(rename = "@ref")]
    pub reference_ref: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupGroupGroupSeparator {
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupSeparator {
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItem {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@icon")]
    pub icon: Option<String>,
    #[serde(rename = "@type")]
    pub item_type: String,
    #[serde(rename = "@preset_name_label")]
    pub preset_name_label: String,
    #[serde(rename = "@name_context")]
    pub name_context: Option<String>,
    #[serde(rename = "@match_expression")]
    pub match_expression: Option<String>,
    #[serde(rename = "$text")]
    pub text_content: Option<String>,
    pub link: PresetsGroupItemLink,
    pub space: Option<Vec<PresetsGroupItemSpace>>,
    pub key: Option<Vec<PresetsGroupItemKey>>,
    pub text: Option<Vec<PresetsGroupItemText>>,
    pub multiselect: Option<PresetsGroupItemMultiselect>,
    pub preset_link: Option<Vec<PresetsGroupItemPresetLink>>,
    pub optional: Option<Vec<PresetsGroupItemOptional>>,
    pub combo: Option<Vec<PresetsGroupItemCombo>>,
    pub checkgroup: Option<Vec<PresetsGroupItemCheckgroup>>,
    pub check: Option<PresetsGroupItemCheck>,
    pub reference: Option<Vec<PresetsGroupItemReference>>,
    pub label: Option<Vec<PresetsGroupItemLabel>>,
    pub roles: Option<PresetsGroupItemRoles>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemLink {
    #[serde(rename = "@wiki")]
    pub wiki: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemSpace {
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemKey {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@value")]
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemText {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@auto_increment")]
    pub auto_increment: Option<String>,
    #[serde(rename = "@match")]
    pub text_match: Option<String>,
    #[serde(rename = "@alternative_autocomplete_keys")]
    pub alternative_autocomplete_keys: Option<String>,
    #[serde(rename = "@use_last_as_default")]
    pub use_last_as_default: Option<String>,
    #[serde(rename = "@text_context")]
    pub text_context: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemMultiselect {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@values")]
    pub values: String,
    #[serde(rename = "@values_searchable")]
    pub values_searchable: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemPresetLink {
    #[serde(rename = "@preset_name")]
    pub preset_name: String,
    #[serde(rename = "@text")]
    pub text: Option<String>,
    #[serde(rename = "@text_context")]
    pub text_context: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemOptional {
    #[serde(rename = "@text")]
    pub text_attr: Option<String>,
    #[serde(rename = "$text")]
    pub text_content: Option<String>,
    pub combo: Option<Vec<PresetsGroupItemOptionalCombo>>,
    pub space: Option<Vec<PresetsGroupItemOptionalSpace>>,
    pub reference: Option<Vec<PresetsGroupItemOptionalReference>>,
    pub check: Option<PresetsGroupItemOptionalCheck>,
    pub checkgroup: Option<PresetsGroupItemOptionalCheckgroup>,
    pub text: Option<Vec<PresetsGroupItemOptionalText>>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemOptionalCombo {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@values")]
    pub values: Option<String>,
    #[serde(rename = "@values_sort")]
    pub values_sort: Option<String>,
    #[serde(rename = "@values_no_i18n")]
    pub values_no_i18n: Option<String>,
    #[serde(rename = "@delimiter")]
    pub delimiter: Option<String>,
    #[serde(rename = "@values_context")]
    pub values_context: Option<String>,
    #[serde(rename = "@match")]
    pub combo_match: Option<String>,
    #[serde(rename = "@use_last_as_default")]
    pub use_last_as_default: Option<String>,
    #[serde(rename = "@values_from")]
    pub values_from: Option<String>,
    #[serde(rename = "@text_context")]
    pub text_context: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemOptionalSpace {
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemOptionalReference {
    #[serde(rename = "@ref")]
    pub reference_ref: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemOptionalCheck {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemOptionalCheckgroup {
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub check: Vec<PresetsGroupItemOptionalCheckgroupCheck>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemOptionalCheckgroupCheck {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemOptionalText {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@match")]
    pub text_match: Option<String>,
    #[serde(rename = "@use_last_as_default")]
    pub use_last_as_default: Option<String>,
    #[serde(rename = "@alternative_autocomplete_keys")]
    pub alternative_autocomplete_keys: Option<String>,
    #[serde(rename = "@regions")]
    pub regions: Option<String>,
    #[serde(rename = "@length")]
    pub length: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemCombo {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@values")]
    pub values: Option<String>,
    #[serde(rename = "@match")]
    pub combo_match: Option<String>,
    #[serde(rename = "@values_context")]
    pub values_context: Option<String>,
    #[serde(rename = "@use_last_as_default")]
    pub use_last_as_default: Option<String>,
    #[serde(rename = "@values_from")]
    pub values_from: Option<String>,
    #[serde(rename = "@default")]
    pub default: Option<String>,
    #[serde(rename = "@values_searchable")]
    pub values_searchable: Option<String>,
    #[serde(rename = "@values_sort")]
    pub values_sort: Option<String>,
    #[serde(rename = "@display_values")]
    pub display_values: Option<String>,
    #[serde(rename = "@length")]
    pub length: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemCheckgroup {
    #[serde(rename = "@columns")]
    pub columns: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub check: Vec<PresetsGroupItemCheckgroupCheck>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemCheckgroupCheck {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@match")]
    pub check_match: String,
    #[serde(rename = "@text_context")]
    pub text_context: Option<String>,
    #[serde(rename = "@regions")]
    pub regions: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemCheck {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@match")]
    pub check_match: Option<String>,
    #[serde(rename = "@disable_off")]
    pub disable_off: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemReference {
    #[serde(rename = "@ref")]
    pub reference_ref: String,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemLabel {
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@text_context")]
    pub text_context: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemRoles {
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub role: Vec<PresetsGroupItemRolesRole>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsGroupItemRolesRole {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@requisite")]
    pub requisite: String,
    #[serde(rename = "@type")]
    pub role_type: String,
    #[serde(rename = "@count")]
    pub count: Option<String>,
    #[serde(rename = "@member_expression")]
    pub member_expression: Option<String>,
}


