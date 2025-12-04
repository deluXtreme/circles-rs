use circles_profiles::{Profile, Profiles};
use std::time::{SystemTime, UNIX_EPOCH};

const PROFILE_SERVICE_URL: &str = "https://rpc.aboutcircles.com/profiles/";
const RUN_LIVE_PROFILE_TESTS: &str = "RUN_LIVE_PROFILE_TESTS";

/// Ignored by default: requires network access and a `PROFILE_CID` environment variable.
#[tokio::test]
#[ignore = "requires PROFILE_CID env var and network access"]
async fn fetch_profile_when_cid_provided() {
    let cid = match std::env::var("PROFILE_CID") {
        Ok(cid) => cid,
        Err(_) => {
            eprintln!("skipping: set PROFILE_CID to run this integration test");
            return;
        }
    };

    let client = Profiles::new(PROFILE_SERVICE_URL).expect("valid profile service url");
    let profile = client.get(&cid).await.expect("profile fetch request");
    let profile = profile.expect("expected profile to exist");
    println!("Fetched profile for {cid}: {profile:?}");
}

/// Ignored by default: hits the live profile service to pin and fetch a profile.
#[tokio::test]
#[ignore = "requires RUN_LIVE_PROFILE_TESTS=1 and network access"]
async fn create_and_fetch_round_trip() {
    if std::env::var(RUN_LIVE_PROFILE_TESTS).ok().as_deref() != Some("1") {
        eprintln!("skipping: set RUN_LIVE_PROFILE_TESTS=1 to run this integration test");
        return;
    }

    let client = Profiles::new(PROFILE_SERVICE_URL).expect("valid profile service url");

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs();
    let profile = Profile {
        // Keep name short (<36 chars) to satisfy service validation
        name: format!("profiles-integration-{ts}"),
        description: Some("integration test profile".to_string()),
        preview_image_url: None,
        image_url: None,
        location: None,
        geo_location: None,
        extensions: None,
    };

    let cid = client.create(&profile).await.expect("profile creation");
    assert!(
        !cid.is_empty(),
        "service returned an empty cid for created profile"
    );

    let fetched = client
        .get(&cid)
        .await
        .expect("profile fetch request")
        .expect("expected freshly created profile to exist");
    assert_eq!(fetched.name, profile.name);
    assert_eq!(fetched.description, profile.description);
    assert_eq!(fetched.preview_image_url, profile.preview_image_url);

    println!("Created + fetched profile: cid={cid}, profile={fetched:?}");
}
