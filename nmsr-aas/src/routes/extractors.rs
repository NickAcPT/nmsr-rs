use super::{
    query::{RenderRequestMultipartParams, RenderRequestQueryParams},
    RenderRequestValidator,
};
use crate::{
    error::{NMSRaaSError, RenderRequestError, Result},
    model::request::{
        entry::RenderRequestEntry, RenderRequest, RenderRequestExtraSettings, RenderRequestMode,
    },
};
use async_trait::async_trait;
use axum::{
    extract::{FromRequest, Path, Query, Request},
    RequestExt,
};
use axum_extra::extract::Multipart;
use hyper::Method;
use is_empty::IsEmpty;
use serde_json::{json, Value};
use std::{borrow::ToOwned, collections::HashMap};

#[async_trait]
impl<S> FromRequest<S> for RenderRequest
where
    S: Send + Sync + RenderRequestValidator,
{
    type Rejection = NMSRaaSError;

    /// Extract a [`RenderRequest`] from the request.
    ///
    /// A [`RenderRequest`] contains an entry and its respective options.
    ///
    /// URLs have the following format:
    ///  - `GET /:mode/:entry?options`
    ///  - `POST /:mode`
    ///
    /// The entry is in the URL path, and the options are in the query string.
    ///
    async fn from_request(mut request: Request, state: &S) -> Result<Self> {
        let (mode, entry, mut query) = if request.method() == Method::POST {
            let Path(mode_str) = request
                .extract_parts_with_state::<Path<String>, S>(state)
                .await
                .map_err(RenderRequestError::from)?;

            let mode = RenderRequestMode::try_from(mode_str.as_str())
                .ok()
                .filter(|r| state.validate_mode(r))
                .ok_or_else(|| RenderRequestError::InvalidRenderMode(mode_str))?;

            let mut multipart = Multipart::from_request(request, state)
                .await
                .map_err(RenderRequestError::from)?;

            let mut data: HashMap<String, Value> = HashMap::new();

            while let Some(field) = multipart
                .next_field()
                .await
                .map_err(RenderRequestError::from)?
            {
                if let Some(name) = field.name().map(ToOwned::to_owned) {
                    let entry_content = if field.content_type().is_none() {
                        let str = field.text().await.map_err(RenderRequestError::from)?;
                        serde_json::from_str(&str).unwrap_or(Value::String(str))
                    } else {
                        Value::from(
                            field
                                .bytes()
                                .await
                                .map_err(RenderRequestError::from)?
                                .to_vec(),
                        )
                    };

                    data.insert(name.clone(), entry_content);
                }
            }

            let object = json!(data);

            let query = serde_json::from_value::<RenderRequestMultipartParams>(object.clone())
                .map_err(|e| RenderRequestError::MultipartDecodeError(e, object.clone()))?;

            let entry = RenderRequestEntry::try_from(query.skin)?;

            (mode, entry, query.query)
        } else {
            let Path((mode_str, entry_str)) = request
                .extract_parts_with_state::<Path<(String, String)>, S>(state)
                .await
                .map_err(RenderRequestError::from)?;

            let mode = RenderRequestMode::try_from(mode_str.as_str())
                .ok()
                .filter(|r| state.validate_mode(r))
                .ok_or_else(|| RenderRequestError::InvalidRenderMode(mode_str))?;

            let entry = RenderRequestEntry::try_from(entry_str)?;

            let Query(query) = request
                .extract_parts_with_state::<Query<RenderRequestQueryParams>, S>(state)
                .await
                .map_err(RenderRequestError::from)?;

            (mode, entry, query)
        };

        query.validate(mode)?;

        let excluded_features = query.get_excluded_features();

        let model = query.get_model();

        let extra_settings = Some(RenderRequestExtraSettings {
            width: query.width,
            height: query.height,

            yaw: query.yaw,
            pitch: query.pitch,
            roll: query.roll,

            arm_rotation: query.arms,
            distance: query.distance,

            x_pos: query.x_pos,
            y_pos: query.y_pos,
            z_pos: query.z_pos,

            helmet: query.helmet,
            chestplate: query.chestplate,
            leggings: query.leggings,
            boots: query.boots,
        })
        .filter(|s| !s.is_empty());

        let mut request = Self::new_from_excluded_features(
            mode,
            entry,
            model,
            excluded_features,
            extra_settings,
        );
        
        state.cleanup_request(&mut request);
        
        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::{debug_handler, extract::State, routing::get, Router, body::Body};
    use enumset::{enum_set, EnumSet};
    use hyper::Request;
    use tokio::sync::mpsc::Sender;
    use tower::ServiceExt;
    use uuid::uuid;

    use crate::{
        model::request::{
            entry::{RenderRequestEntry, RenderRequestEntryModel},
            RenderRequest, RenderRequestFeatures, RenderRequestMode,
        },
        routes::RenderRequestValidator,
    };

    impl RenderRequestValidator for Sender<RenderRequest> {
        fn validate_mode(&self, _mode: &RenderRequestMode) -> bool {
            true
        }
    }

    #[debug_handler]
    async fn test_handler(State(state): State<Sender<RenderRequest>>, request: RenderRequest) {
        state.send(request).await.unwrap();
    }

    async fn render_request_from_url(url: &str) -> RenderRequest {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<RenderRequest>(1);

        let request = Request::builder()
            .uri(url)
            .body(Body::empty())
            .expect("Failed to build request");

        let app: Router = Router::new()
            .route("/:mode/:entry", get(test_handler))
            .with_state(tx);

        app.oneshot(request).await.expect("Failed to send request");

        rx.recv().await.expect("Failed to receive request")
    }

    #[tokio::test]
    async fn test_render_request_from_request_parts() {
        let entry =
            RenderRequestEntry::MojangPlayerUuid(uuid!("ad4569f3-7576-4376-a7c7-8e8cfcd9b832"));

        let expected = HashMap::from([
            (
                "http://localhost:8621/skin/ad4569f3-7576-4376-a7c7-8e8cfcd9b832",
                RenderRequest {
                    mode: RenderRequestMode::Skin,
                    entry: entry.clone(),
                    model: None,
                    features: EnumSet::only(RenderRequestFeatures::UnProcessedSkin),
                    extra_settings: None
                },
            ),
            (
                "http://localhost:8621/skin/ad4569f3-7576-4376-a7c7-8e8cfcd9b832?no=shadow",
                RenderRequest {
                    mode: RenderRequestMode::Skin,
                    entry: entry.clone(),
                    model: None,
                    features: EnumSet::only(RenderRequestFeatures::UnProcessedSkin),
                    extra_settings: None
                },
            ),
            (
                "http://localhost:8621/skin/ad4569f3-7576-4376-a7c7-8e8cfcd9b832?alex&noshading&nolayers",
                RenderRequest {
                    mode: RenderRequestMode::Skin,
                    entry: entry.clone(),
                    model: Some(RenderRequestEntryModel::Alex),
                    features: EnumSet::only(RenderRequestFeatures::UnProcessedSkin),
                    extra_settings: None
                },
            ),
            (
                "http://localhost:8621/fullbody/ad4569f3-7576-4376-a7c7-8e8cfcd9b832?nolayers&no=cape",
                RenderRequest {
                    mode: RenderRequestMode::FullBody,
                    entry: entry.clone(),
                    model: None,
                    features: EnumSet::all().difference(enum_set!(RenderRequestFeatures::BodyLayers | RenderRequestFeatures::HatLayer | RenderRequestFeatures::Cape | RenderRequestFeatures::UnProcessedSkin | RenderRequestFeatures::Custom | RenderRequestFeatures::ExtraSettings)),
                    extra_settings: None
                },
            ),
        ]);

        for (url, element) in expected {
            let result = render_request_from_url(url).await;

            assert_eq!(element, result, "Failed to extract for url: {url}");
        }
    }
}
