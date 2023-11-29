use std::collections::HashMap;

use futures_util::StreamExt;
use rspotify::{model::TrackId, prelude::*, scopes, AuthCodeSpotify, Config, Credentials, OAuth};

#[tokio::main]
async fn main() {
    // You can use any logger for debugging.
    env_logger::init();

    let spotify = init_spotify();

    // Obtaining the access token
    let url = spotify.get_authorize_url(false).unwrap();
    // This function requires the `cli` feature enabled.
    spotify.prompt_for_token(&url).await.unwrap();

    // todo use streams, as this is awefully slow
    let all_saved_tracks = spotify
        .current_user_saved_tracks(None)
        .take(2000) // todo to delete
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .filter(|track| track.is_ok());

    println!("saved tracks");

    let mut genre_tracks: HashMap<String, Vec<TrackId<'static>>> = HashMap::new();

    // Collect and save a list of all genre for each track's artists into a Hashmap
    for track_data in all_saved_tracks.into_iter().take(2932) {
        let track = track_data.unwrap().track;

        let track_id = track.id.unwrap();

        let taid = track.artists.first().unwrap().id.clone().unwrap();

        // todo here I can keep a reference of the passed through artists .. 
        let genres = spotify.artist(taid).await.unwrap().genres;

        for genre in genres {
            genre_tracks
                .entry(genre)
                .or_insert(Vec::new())
                .push(track_id.clone());
        }
    }
    println!("track data");

    // for genres in genre_tracks.clone() {
    //     println!("* Genre [{}] | Song count: {}", genres.0, genres.1.len())
    // }
    let user_id = spotify.me().await.unwrap().id;
    let playlist = spotify
        .user_playlist_create(
            user_id,
            "Auto Generated uk dance Playlist",
            Some(true),
            Some(false),
            Some("Auto-Generated playlist containing music from the same genre"),
        )
        .await
        .unwrap();

    let some_french_hip_hop_tracks = genre_tracks
        .get("uk dance")
        .unwrap()
        .iter()
        .map(|track_id| PlayableId::Track(track_id.clone()))
        .collect::<Vec<PlayableId>>();
    println!("Some");
    let _ = spotify
        .playlist_add_items(playlist.id, some_french_hip_hop_tracks, None)
        .await;
}

fn init_spotify() -> AuthCodeSpotify {
    let config = Config {
        //token_cached: true,
        //cache_path: create_cache_path_if_absent(jar),
        ..Default::default()
    };

    // Please notice that protocol of redirect_uri, make sure it's http (or
    // https). It will fail if you mix them up.
    let oauth = OAuth {
        scopes: scopes!(
            "user-read-email",
            "user-read-private",
            "user-top-read",
            "user-read-recently-played",
            "user-follow-read",
            "user-library-read",
            "user-read-currently-playing",
            "user-read-playback-state",
            "user-read-playback-position",
            "playlist-read-collaborative",
            "playlist-read-private",
            "user-follow-modify",
            "user-library-modify",
            "user-modify-playback-state",
            "playlist-modify-public",
            "playlist-modify-private",
            "ugc-image-upload"
        ),
        redirect_uri: "https://example.com/callback".to_owned(),
        ..Default::default()
    };

    let creds = Credentials::from_env().unwrap();
    AuthCodeSpotify::with_config(creds, oauth, config)
}
