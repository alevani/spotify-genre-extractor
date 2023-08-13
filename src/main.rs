use std::collections::HashMap;

use futures_util::StreamExt;
use rspotify::{
    model::{ArtistId, SavedTrack, TimeRange, TrackId},
    prelude::*,
    scopes, AuthCodeSpotify, Config, Credentials, OAuth, Token,
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

    // todo use streams, as this is awefully slow
    let all_saved_tracks = spotify
        .current_user_saved_tracks(None)
        .take(200) // todo to delete
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .filter(|track| track.is_ok());

    let mut genre_tracks: HashMap<String, Vec<TrackId<'static>>> = HashMap::new();

    // Collect and save a list of all genre for each track's artists into a Hashmap
    for track_data in all_saved_tracks.into_iter() {
        let track = track_data
        .unwrap()
        .track;

        let track_id = track.id.unwrap();

        let taid = track
            .artists
            .first()
            .unwrap()
            .id
            .clone()
            .unwrap();
        let genres = spotify.artist(taid).await.unwrap().genres;
        
        for genre in genres {
            genre_tracks.entry(genre).or_insert(Vec::new()).push(track_id.clone());
        }
    }

    println!("{genre_tracks:?}");

    // stream
    //     .try_for_each_concurrent(10, |item| async move {

    //         // let taid = item.track.artists.first().unwrap().id.unwrap();
    //         if let Some(artist) = item.track.artists.first().take() {
    //             taids.push(artist.id.unwrap());
    //         }

    //         // if let Some(id) = taid {
    //         //     println!("{:?}", spotify.artist(ids).await.unwrap().genres);
    //         // }

    //         Ok(())
    //     })
    //     .await
    //     .unwrap();
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
