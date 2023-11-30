use std::io;
use std::ops::Deref;
use std::{collections::HashMap, sync::Arc};
use tokio::time::{sleep, Duration};

use futures_retry::{RetryPolicy, StreamRetryExt};

use futures_util::lock::Mutex;
use futures_util::StreamExt;
use rspotify::model::ArtistId;
use rspotify::{model::TrackId, prelude::*, scopes, AuthCodeSpotify, Config, Credentials, OAuth};

async fn fetch_artist_genres_with_retry(
    spotify: &AuthCodeSpotify,
    aid: &ArtistId<'_>,
) -> Vec<String> {
    let mut result = spotify.artist(aid.to_owned()).await;

    while result.is_err() {
        println!("Hitting spotify API rate limit .. sleeping 80s");
        sleep(Duration::from_millis(80000)).await;
        result = spotify.artist(aid.to_owned()).await;
    }

    result.unwrap().genres
}

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
            print!(".");
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

    let artist_ids_locked = artist_ids.lock().await;

    println!("\nCurrent saved tracks fetched.");
    println!(
        "Retrieving genre by artist for {} artists ..",
        artist_ids_locked.keys().count()
    );

    for (index, (aid, vtid)) in artist_ids_locked.iter().enumerate() {
        println!(
            "[{index}/{}] Artist: {aid}",
            artist_ids_locked.keys().count()
        );

        let genre = fetch_artist_genres_with_retry(&spotify, aid).await;
        let genre_iter = genre.iter();

        if genre_iter.count() == 0 {
            println!(" - Unknown genre");
            genre_tracks
                .entry("unknown genre".to_string())
                .or_default()
                .append(&mut vtid.clone())
        } else {
            for genre in fetch_artist_genres_with_retry(&spotify, aid).await {
                println!(" - {genre}");
                genre_tracks
                    .entry(genre)
                    .or_default()
                    .append(&mut vtid.clone())
            }
        }
    }

    println!(".. Done !");

    for genres in genre_tracks.iter() {
        println!(
            "* Genre [{}] | Song count: {} <> ",
            genres.0,
            genres.1.len()
        )
    }

    // let mut input = String::new();
    // let _ = io::stdin().read_line(&mut input);
    let input = "french indie pop".to_string();

    let user_id = spotify.me().await.unwrap().id;
    let playlist = spotify
        .user_playlist_create(
            user_id,
            &format!("Programatically generated [{}] Playlist", input.to_uppercase()),
            Some(true),
            Some(false),
            Some(&format!("This playlist contains all the [{}] songs extracted from a liked playlist. The content may not only contain the same genre, as Spotify attaches a genre to an artist, and not to a song, making it difficult to properly sort.", input.to_uppercase())),
        )
        .await
        .unwrap();

    // This is equivalent to chunking, however the chunk API has limit and I ain't running no nightly here
    let mut track_count = 0;
    let mut tracks: Vec<Vec<PlayableId>> = Vec::new();
    let mut track_record: Vec<PlayableId> = Vec::new();

    for track_id in genre_tracks.get(&input).unwrap() {
        if track_count == 99 {
            track_count = 0;
            tracks.push(track_record);
            track_record = Vec::new();
        }

        track_record.push(PlayableId::Track(track_id.clone()));
        track_count += 1;
    }

    for chunk in tracks {
        let _ = spotify
            .playlist_add_items(playlist.id.clone(), chunk, None)
            .await;
    }
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
