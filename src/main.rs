use std::collections::HashMap;
use std::io;

use rspotify::{
    model::TrackId,
    prelude::*,
    scopes, AuthCodeSpotify, Config, Credentials, OAuth,
};

#[tokio::main]
async fn main() {
    // You can use any logger for debugging.
    env_logger::init();

    let spotify = init_spotify();

    // Obtaining the access token
    let url = spotify.get_authorize_url(false).unwrap();

    // This function requires the `cli` feature enabled.
    spotify.prompt_for_token(&url).await.unwrap();

    let mut i = 0;
    let mut genre_tracks: HashMap<String, Vec<TrackId<'static>>> = HashMap::new();

    // I should use concurrency here and just poll x pages at the same time
    while let Ok(paginated_tracks) = spotify
        .current_user_saved_tracks_manual(None, Some(100), Some(i))
        .await
    {
        for t in paginated_tracks.items {
            let track = t.track;
            let track_id = track.id.unwrap();

            let artist_id = track.artists.first().unwrap().id.clone().unwrap();

            spotify
                .artist(artist_id)
                .await
                .unwrap()
                .genres
                .into_iter()
                .for_each(|genre| {
                    genre_tracks
                        .entry(genre)
                        .or_default()
                        .push(track_id.clone())
                });
        }
        i += 100;
    }

    for genres in genre_tracks.clone() {
        println!("* Genre [{}] | Song count: {}", genres.0, genres.1.len())
    }

    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);

    let user_id = spotify.me().await.unwrap().id;
    let playlist = spotify
        .user_playlist_create(
            user_id,
            &format!("Auto Generated <{input}> Playlist"),
            Some(true),
            Some(false),
            Some("Auto-Generated playlist containing music from the same genre"),
        )
        .await
        .unwrap();

    let tracks = genre_tracks
        .get(&input)
        .unwrap()
        .iter()
        .map(|track_id| PlayableId::Track(track_id.clone()))
        .collect::<Vec<PlayableId>>();
    
    let _ = spotify
        .playlist_add_items(playlist.id, tracks, None)
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
