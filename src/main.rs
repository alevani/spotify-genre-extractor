use serde::{Deserialize, Serialize};
use serde_json::error;
use std::fs::{create_dir, File};
use std::io::{BufReader, BufWriter};
use std::{collections::HashMap, sync::Arc};
use tokio::time::{sleep, Duration};

use futures_util::lock::Mutex;
use futures_util::StreamExt;
use rspotify::model::{ArtistId, FullTrack};
use rspotify::{model::TrackId, prelude::*, scopes, AuthCodeSpotify, Config, Credentials, OAuth};

fn load_saved_data() -> Vec<Artist> {
    let mut current_path = dirs::cache_dir().expect("Failed to locate cache dir");
    current_path.push("playlist-maker");

    if !current_path.as_path().exists() {
        create_dir(current_path.as_path()).expect("Failed to create cachedir");
    }

    current_path.set_file_name("metadata.json");

    let f = File::open(current_path).expect("Failed to open cached OAuth2 File");
    let reader = BufReader::new(f);

    let artists: Result<Vec<Artist>, error::Error> = serde_json::from_reader(reader);
    artists.unwrap()
}

fn save_data(artists: &[Artist]) {
    let mut current_path = dirs::cache_dir().expect("Failed to locate cache dir");
    current_path.push("playlist-maker");

    if !current_path.as_path().exists() {
        create_dir(current_path.as_path()).expect("Failed to create cachedir");
    }

    current_path.set_file_name("metadata.json");

    let f = File::create(current_path).expect("Failed to create metadata.json file");
    let writer = BufWriter::new(f);

    serde_json::to_writer(writer, artists).expect("Failed to write to metadata.json");
}

#[derive(Debug, Deserialize, Serialize)]
struct Artist {
    pub id: ArtistId<'static>,
    pub genres: Vec<String>,
    pub tracks: Vec<FullTrack>,
}

impl Artist {
    pub fn set_genres(&mut self, genres: Vec<String>) {
        self.genres = genres;
    }
}

async fn get_metadata(should_sync: bool, spotify: &AuthCodeSpotify) -> Vec<Artist> {
    if should_sync {
        // Create a shared state for genre_tracks
        let artistsids_and_tracks: Arc<Mutex<HashMap<ArtistId, Vec<FullTrack>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        println!("Fetching current saved tracks ..");
        spotify
            .current_user_saved_tracks(None)
            .for_each_concurrent(None, |t| {
                print!(".");
                let artistsids_and_tracks = Arc::clone(&artistsids_and_tracks); // Clone the Arc for the shared state

                async move {
                    if let Ok(track_data) = t {
                        let track = track_data.track;
                        let artist_id = track.artists.first().unwrap().id.clone().unwrap();

                        artistsids_and_tracks
                            .lock()
                            .await
                            .entry(artist_id)
                            .or_default()
                            .push(track);
                    }
                }
            })
            .await;

        let artistsids_and_tracks_locked = artistsids_and_tracks.lock().await;

        let mut artists: Vec<Artist> = artistsids_and_tracks_locked
            .iter()
            .map(|e| Artist {
                id: e.0.clone(),
                genres: Vec::new(),
                tracks: e.1.clone(),
            })
            .collect();

        println!("\nCurrent saved tracks fetched.");
        println!(
            "Retrieving genre by artist for {} artists ..",
            artistsids_and_tracks_locked.keys().count()
        );

        let n_artists = artists.len();
        for (index, artist) in artists.iter_mut().enumerate() {
            println!("[{index}/{n_artists}] Artist: {}", artist.id);

            let genres = fetch_artist_genres_with_retry(spotify, &artist.id).await;
            let genres_iter = genres.iter();

            if genres_iter.clone().count() == 0 {
                println!(" - unknown genre");
                artist.set_genres(vec!["unknown genre".to_string()]);
            } else {
                genres.iter().for_each(|g| println!("{g}"));
                artist.set_genres(genres);
            }
        }

        println!(".. Done !");

        save_data(&artists);
        artists
    } else {
        load_saved_data()
    }
}

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

    let artists = get_metadata(false, &spotify).await;

    let mut genre_tracks: HashMap<String, Vec<FullTrack>> = HashMap::new();

    artists.iter().for_each(|a| {
        for genre in a.genres.clone() {
            genre_tracks
                .entry(genre)
                .or_default()
                .append(&mut a.tracks.clone());
        }
    });

    for genres in genre_tracks.iter() {
        println!(
            "* Genre [{}] | Song count: {} <> ",
            genres.0,
            genres.1.len()
        )
    }

    // let mut input = String::new();
    // let _ = io::stdin().read_line(&mut input);
    let input = "danish pop".to_string();

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

    for track in genre_tracks.get(&input).unwrap() {
        if track_count == 99 {
            track_count = 0;
            tracks.push(track_record);
            track_record = Vec::new();
        }

        track_record.push(PlayableId::Track(track.id.as_ref().unwrap().clone()));
        track_count += 1;
    }

    tracks.push(track_record);

    for chunk in tracks {
        println!("Adding tracks to the playlist..");
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
