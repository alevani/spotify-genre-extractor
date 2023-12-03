# Playlist-Maker

**Description:**
Playlist-Maker is a command-line tool written in Rust for creating custom Spotify playlists. The tool allows you to extract any genre from your liked playlist and create a new playlist based on that genre.

**Usage:**

1. **Set Up Spotify API:**
   - Go to [Spotify Developer Dashboard](https://developer.spotify.com/dashboard) and create a new API.
   - During API creation, set the callback URL to "https://example.com/callback".
   - Copy your Spotify client secret and ID to the `.env` file.

2. **Syncing Data:**
   - So far, the program calls the genre API for each artist, which, in my case, is awfully slow and may make you run into a 429 TOO MANY request error. I have added some very simple retry logic, for 80s as it's usually 80s..However, you should not have an issue running the program the first time, it only gets tricky if you have too many songs, or sync too many times without saving your metadatas.
   - To avoid issues, consider saving metadata. In `main.rs`, find the line:
     ```Rust
     let artists = get_metadata(false, &spotify).await;
     ```
     - Set the boolean to `true` to load data from the cache, reducing load time. Run the program once with `false`, then switch to `true` for subsequent syncs.

3. **Selecting Playlist Genre:**
   - Update the code in `main.rs`:
     ```Rust
     let input = "danish pop".to_string();
     ```

**Running the Program:**
- Once the environment is set up, run the command `cargo run`.

**Note:**
- The Spotify API provides genres per artist, not per song, making sorting tricky due to artists having multiple genres.
- Consider saving metadata to reduce load times and prevent API request errors.
- Update the playlist genre by modifying the code in `main.rs`.

**Example:**
```Rust
let input = "your-desired-genre".to_string();
```

Feel free to contribute and enhance the functionality of Playlist-Maker!