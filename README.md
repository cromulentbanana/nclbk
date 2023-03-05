A Nextcloud Bookmark API Client.

Currently a first draft implementation, not stable or ready for consumption. Feedback and contributions welcome.

# Run Tests
rust stable >=1.46

`cargo test`

# Development build/run
rust stable >=1.46

`cargo-watch -x run`

Currently nothing to build, yet.

# Building for Production
rust stable >=1.46

`cargo build --release`

Currently nothing to build, yet.

# Example

```rust
use libnclbk;
use url::Url;        

fn main() {
    let auth_id: String = "<your user>".to_owned();
    let base_url: Url = "https://<your nextcloud>".to_owned().parse().unwrap();
    // Create an `app password` at https://<your nextcloud>/index.php/settings/user/security and export it as an env var
    let key: &str = "NC_AUTH_SECRET";
    let auth_secret: String = std::env::var(key).to_owned().unwrap();

    let bookmark_client = libnclbk::BookmarkAPIClient::new(auth_id, auth_secret, base_url).unwrap();
    println!("{:?}", bookmark_client);
} 
```

# Useful Documentation

* [Nextcloud Bookmark API](https://nextcloud-bookmarks.readthedocs.io/en/latest/bookmark.html)
* https://redbeardlab.com/2019/05/07/rust-and-glibc-version/
