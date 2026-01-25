import { Schema } from "effect";

const ExecutablesSchema = Schema.Struct({
  executable: Schema.Array(Schema.String),
});

const LaunchOptionsSchema = Schema.Struct({
  option: Schema.Array(Schema.String),
});

const ClientPartsSchema = Schema.Struct({
  client_part: Schema.Array(Schema.String),
});

const ClientTypeSchema = Schema.Struct({
  initial_app_type: Schema.String,
  final_app_type: Schema.String,
  min_supported_os: Schema.String,
  client_parts: ClientPartsSchema,
});

const ClientTypesSchema = Schema.Struct({
  client_type: Schema.Array(ClientTypeSchema),
});

const DemoAccountsSchema = Schema.Struct({
  enabled: Schema.Boolean,
});

const OptionsSchema = Schema.Struct({
  option: Schema.Tuple(Schema.Boolean, Schema.String, Schema.String, Schema.String),
});

const FeaturesSchema = Schema.Struct({
  feature: Schema.Array(Schema.String),
});

const CustomSupportLinksSchema = Schema.Struct({
  link: Schema.String,
});

const CommonCommerceAgreementsSchema = Schema.Struct({
  agreement: Schema.String,
});

const InstanceSchema = Schema.Struct({
  platform_realm_id: Schema.String,
  game_id: Schema.String,
  title_id: Schema.String,
  content_service_url: Schema.String,
  options: OptionsSchema,
  features: FeaturesSchema,
  custom_support_links: CustomSupportLinksSchema,
  common_commerce_agreements: CommonCommerceAgreementsSchema,
});

const InstancesSchema = Schema.Struct({
  instance: InstanceSchema,
});

const RealmsSettingsSchema = Schema.Struct({
  instances: InstancesSchema,
});

const ItemSchema = Schema.Struct({
  text: Schema.Array(Schema.String),
});

const LocalizationSchema = Schema.Struct({
  item: Schema.Array(ItemSchema),
});

const PredefinedSectionSchema = Schema.Struct({
  app_id: Schema.String,
  chain_id: Schema.String,
  supported_languages: Schema.String,
  default_language: Schema.String,
  name: Schema.String,
  fs_name: Schema.String,
  shortcut_name: Schema.String,
  executables: ExecutablesSchema,
  launch_options: LaunchOptionsSchema,
  environment_name: Schema.String,
  keep_patches_interval: Schema.Number,
  client_types: ClientTypesSchema,
  login_enabled: Schema.Boolean,
  demo_accounts: DemoAccountsSchema,
  realms_settings: RealmsSettingsSchema,
  localization: LocalizationSchema,
});

const GeneratedSectionSchema = Schema.Struct({
  launch_forbidden: Schema.String,
  stop_supporting: Schema.String,
});

const ProtocolSchema = Schema.Struct({
  version: Schema.Number,
  predefined_section: PredefinedSectionSchema,
  generated_section: GeneratedSectionSchema,
});

export const GetMetadataResponseSchema = Schema.Struct({
  protocol: ProtocolSchema,
});
