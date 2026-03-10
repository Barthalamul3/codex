use crate::features::Feature;
use crate::features::Features;
use codex_protocol::models::ImageDetail;
use codex_protocol::openai_models::ModelInfo;

pub(crate) fn can_request_original_image_detail(
    features: &Features,
    model_info: &ModelInfo,
) -> bool {
    model_info.supports_image_detail_original
        && (features.enabled(Feature::ImageDetailOriginal)
            || features.enabled(Feature::ImageDetailOriginalAlways))
}

pub(crate) fn should_force_original_image_detail(
    features: &Features,
    model_info: &ModelInfo,
) -> bool {
    can_request_original_image_detail(features, model_info)
        && features.enabled(Feature::ImageDetailOriginalAlways)
}

pub(crate) fn normalize_output_image_detail(
    features: &Features,
    model_info: &ModelInfo,
    detail: Option<ImageDetail>,
) -> Option<ImageDetail> {
    // `image_detail_original_always` intentionally preserves the legacy
    // always-on behavior by forcing original detail whenever the model
    // supports it, even if the caller requested a lower detail value.
    if should_force_original_image_detail(features, model_info) {
        return Some(ImageDetail::Original);
    }

    match detail {
        Some(ImageDetail::Original) if !can_request_original_image_detail(features, model_info) => {
            None
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::config::test_config;
    use crate::features::Features;
    use crate::models_manager::manager::ModelsManager;
    use pretty_assertions::assert_eq;

    #[test]
    fn image_detail_original_feature_enables_explicit_original_without_force() {
        let config = test_config();
        let mut model_info =
            ModelsManager::construct_model_info_offline_for_tests("gpt-5-codex", &config);
        model_info.supports_image_detail_original = true;
        let mut features = Features::with_defaults();
        features.enable(Feature::ImageDetailOriginal);

        assert!(can_request_original_image_detail(&features, &model_info));
        assert!(!should_force_original_image_detail(&features, &model_info));
        assert_eq!(
            normalize_output_image_detail(&features, &model_info, Some(ImageDetail::Original)),
            Some(ImageDetail::Original)
        );
        assert_eq!(
            normalize_output_image_detail(&features, &model_info, None),
            None
        );
    }

    #[test]
    fn image_detail_original_always_feature_forces_original() {
        let config = test_config();
        let mut model_info =
            ModelsManager::construct_model_info_offline_for_tests("gpt-5-codex", &config);
        model_info.supports_image_detail_original = true;
        let mut features = Features::with_defaults();
        features.enable(Feature::ImageDetailOriginalAlways);

        assert!(can_request_original_image_detail(&features, &model_info));
        assert!(should_force_original_image_detail(&features, &model_info));
        assert_eq!(
            normalize_output_image_detail(&features, &model_info, None),
            Some(ImageDetail::Original)
        );
        assert_eq!(
            normalize_output_image_detail(&features, &model_info, Some(ImageDetail::Low)),
            Some(ImageDetail::Original)
        );
    }

    #[test]
    fn explicit_original_is_dropped_without_feature_or_model_support() {
        let config = test_config();
        let mut model_info =
            ModelsManager::construct_model_info_offline_for_tests("gpt-5-codex", &config);
        model_info.supports_image_detail_original = true;
        let features = Features::with_defaults();

        assert_eq!(
            normalize_output_image_detail(&features, &model_info, Some(ImageDetail::Original)),
            None
        );

        let mut features = Features::with_defaults();
        features.enable(Feature::ImageDetailOriginal);
        model_info.supports_image_detail_original = false;
        assert_eq!(
            normalize_output_image_detail(&features, &model_info, Some(ImageDetail::Original)),
            None
        );
    }
}
