use std::io;
use std::{collections::HashMap, sync::Arc};

use futures_retry::{RetryPolicy, StreamRetryExt};

use futures_util::lock::Mutex;
use futures_util::StreamExt;
use rspotify::model::ArtistId;
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

    // Create a shared state for genre_tracks
    let mut genre_tracks: HashMap<String, Vec<TrackId>> = HashMap::new();

    // Create a shared state for genre_tracks
    let artist_ids: Arc<Mutex<HashMap<ArtistId, Vec<TrackId>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    println!("Fetching current saved tracks ..");
    spotify
        .current_user_saved_tracks(None)
        .for_each_concurrent(None, |t| {
            let artist_ids = Arc::clone(&artist_ids); // Clone the Arc for the shared state

            async move {
                if let Ok(track_data) = t {
                    let track = track_data.track;
                    let track_id = track.id.unwrap();
                    let artist_id = track.artists.first().unwrap().id.clone().unwrap();

                    artist_ids
                        .lock()
                        .await
                        .entry(artist_id)
                        .or_default()
                        .push(track_id);
                }
            }
        })
        .await;
    
    println!("Current saved tracks fetched.");
    println!("Retrieving genre by artist ..");
    // ¯\_(ツ)_/¯
    let artist_ids_locked = artist_ids.lock().await;
    for (aid, vtid) in artist_ids_locked.iter() {
        for genre in spotify.artist(aid.to_owned()).await.unwrap().genres {
            genre_tracks
                .entry(genre)
                .or_default()
                .append(&mut vtid.clone())
        }
    }

    println!(".. Done !");

    for genres in genre_tracks.iter() {
        print!(
            "* Genre [{}] | Song count: {} <> ",
            genres.0,
            genres.1.len()
        )
    }

    panic!();

    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);

    let user_id = spotify.me().await.unwrap().id;
    let playlist = spotify
        .user_playlist_create(
            user_id,
            &format!("Programatically generated [{}] Playlist", input.to_uppercase()),
            Some(true),
            Some(false),
            Some("Programatically generated playlist containing music from the same genre .. work in progress. It does have some major issues sorting genres as Spotify only attach the genre to an artist, a not the song itself."),
        )
        .await
        .unwrap();

    let tracks = genre_tracks
        .get(&input)
        .unwrap()
        .iter()
        .map(|track_id| PlayableId::Track(track_id.clone()))
        .collect::<Vec<PlayableId>>();

    let _ = spotify.playlist_add_items(playlist.id, tracks, None).await;
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
