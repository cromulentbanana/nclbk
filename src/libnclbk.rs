//Consider using https://crates.io/crates/thiserror
use anyhow::Result;
use base64::encode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use url::Url;

#[derive(Debug, Deserialize, Serialize)]
struct BookmarksResponse {
    data: Vec<Option<Bookmark>>,
    status: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct TagsResponse {
    data: Vec<String>,
    status: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Bookmark {
    added: u64,
    clickcount: u64,
    description: String,
    folders: Vec<i32>,
    id: u64,
    lastmodified: u64,
    public: Option<u64>,
    tags: Vec<String>,
    title: String,
    url: String,
    user_id: Option<String>,
}

#[derive(Debug)]
pub struct BookmarkAPIClient {
    auth_id: String,
    auth_secret: String,
    root_url: Url,
    bookmarks_url: Url,
    tags_url: Url,
    client: reqwest::Client,
}

impl BookmarkAPIClient {
    pub fn new(auth_id: String, auth_secret: String, root_url: Url) -> Result<BookmarkAPIClient> {
        let base_url = root_url.join("/index.php/apps/bookmarks/public/rest/v2")?;
        let bookmarks_url = base_url.join("/bookmark")?;
        let tags_url = base_url.join("/tag")?;

        Ok(BookmarkAPIClient {
            auth_id,
            auth_secret,
            root_url,
            bookmarks_url,
            tags_url,
            client: reqwest::Client::new(),
        })
    }

    pub async fn read_tags(&self) -> Result<Vec<String>> {
        let request_url = format!("{}", &self.tags_url);

        log::debug!("tag api url: {}", request_url);
        let encoded_basic_auth = encode(format!("{}:{}", self.auth_id, self.auth_secret));

        let client = reqwest::Client::new();
        log::debug!("calling get");
        let response = client
            .get(request_url)
            .header("AUTHORIZATION", format!("Basic {}", encoded_basic_auth))
            .send()
            .await?;

        let response_text = response.text().await?;
        log::debug!("Response Text: {}", &response_text);

        let mut tags_response: Vec<String> = serde_json::from_str(&response_text)?;
        tags_response.sort();

        if tags_response.is_empty() {
            log::debug!("No tags exist.")
        }

        Ok(tags_response)
    }

    pub async fn read_bookmarks(
        &self,
        query_tags: Vec<String>,
        filters: Vec<String>,
        unavailable: bool,
    ) -> Result<Vec<Bookmark>> {
        let tags: String = query_tags
            .clone()
            .into_iter()
            .map(|tag| format!("tags[]={}", tag))
            .collect::<Vec<String>>()
            .join("&");

        let filter: String = filters
            .clone()
            .into_iter()
            .map(|x| format!("search[]={}", x))
            .collect::<Vec<String>>()
            .join("&");

        let page: String = "page=-1".to_string();
        let conjunction: String = "conjunction=or".to_string();
        let unavailable: String = format!("unavailable={}", unavailable);

        //https://github.com/nextcloud/bookmarks/blob/4711d6507fd3e736fe15b104c7bbe54d276fac5b/lib/QueryParameters.php
        let request_url = format!(
            "{bookmarks_url}?{parameters}",
            bookmarks_url = self.bookmarks_url,
            parameters = vec![tags, filter, page, conjunction, unavailable].join("&"),
        );

        log::info!("bookmark api url: {}", request_url);
        let encoded_basic_auth = encode(format!("{}:{}", self.auth_id, self.auth_secret));

        let client = reqwest::Client::new();
        log::debug!("calling get");
        // Failing here
        let response = client
            .get(&request_url)
            .header("AUTHORIZATION", format!("Basic {}", encoded_basic_auth))
            .send()
            .await?;

        let response_text = response.text().await?;
        log::debug!("Response Text: {}", &response_text);

        let bookmarks_response: BookmarksResponse = serde_json::from_str(&response_text)?;

        if bookmarks_response.data.is_empty() {
            log::info!("No bookmarks matched the query selector(s).")
        }

        //TODO find a bettery approach than using `unwrap()` here to avoid panics
        let bookmarks: Vec<Bookmark> = bookmarks_response
            .data
            .into_iter()
            .map(|b| b.unwrap_or_default())
            .collect();

        Ok(bookmarks)
    }

    // TODO: Consider better approaches here: subprocess, show some progress, handle errors better,
    pub fn download_url(
        &self,
        url: &str,
        path: Option<&PathBuf>,
        command: &String,
    ) -> Result<bool> {
        std::fs::create_dir_all(path.unwrap_or(&PathBuf::from("./"))).unwrap();
        let mut child = Command::new(command)
            .current_dir(path.unwrap_or(&PathBuf::from("./")))
            .arg("-i")
            .arg(url)
            .spawn()
            .expect("Failed to execute command");

        let ecode = child.wait().expect("Failed to wait on child");

        log::debug!("output: {:?}", ecode);
        Ok(ecode.success())
    }

    pub async fn delete_bookmark(&self, id: u64) -> Result<bool, reqwest::Error> {
        let request_url = format!(
            "{bookmarks_url}/{id}",
            bookmarks_url = self.bookmarks_url,
            id = id,
        );

        let encoded_basic_auth = encode(format!("{}:{}", self.auth_id, self.auth_secret));
        let response = self
            .client
            .delete(&request_url)
            .header("AUTHORIZATION", "Basic {}".to_owned() + &encoded_basic_auth)
            .send()
            .await?;

        let status = response.status().is_success();
        let response_text = response.text().await?;
        log::info!("delete api response: {}", response_text);
        Ok(status)
    }

    pub async fn run(
        &self,
        command: String,
        query_tags: Vec<String>,
        filters: Vec<String>,
        unavailable: bool,
        do_download: bool,
        do_remove_bookmark: bool,
        output_dir: Option<PathBuf>,
    ) -> Result<()> {
        let bookmarks = self
            .read_bookmarks(query_tags, filters, unavailable)
            .await?;

        for bookmark in bookmarks {
            log::debug!("Bookmark: {:?}", bookmark);
            let url: String = bookmark.url.to_string();
            // FIXME: Find a more elegant way to unquote the URL
            let url = url
                .trim_start_matches('"')
                .to_owned()
                .trim_end_matches('"')
                .to_owned();
            log::info!("bookmark url: {}", url);

            let download_success = if do_download {
                self.download_url(&url, output_dir.as_ref(), &command)
                    .unwrap()
            } else {
                true
            };

            if download_success && do_remove_bookmark {
                self.delete_bookmark(bookmark.id).await?;
                log::info!("Removed Bookmark: {}\n{}", bookmark.title, url);
            } else {
                log::info!("Would have deleted url: {}", url);
            }
        }

        Ok(())
    }
}

//https://doc.rust-lang.org/book/ch11-00-testing.html
#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use httpmock::Mock;

    fn base_url(server: &MockServer) -> String {
        return server
            .url("/index.php/apps/bookmarks/public/rest/v2")
            .to_string();
    }

    fn get_api_client(mock_server: &MockServer) -> Result<BookmarkAPIClient> {
        BookmarkAPIClient::new(
            String::from("auth_id"),
            String::from("auth_Secret"),
            Url::parse(&base_url(&mock_server))?,
        )
    }

    #[test]
    fn bookmark_api_client_should_have_expected_urls() -> Result<()> {
        let server: MockServer = MockServer::start();
        let base_url = &base_url(&server);

        let client = get_api_client(&server)?;
        let expected_bookmarks_url = Url::parse(base_url)?.join("/bookmark")?;
        let expected_tags_url = Url::parse(base_url)?.join("/tag")?;
        assert!(client.bookmarks_url == expected_bookmarks_url);
        assert!(client.tags_url == expected_tags_url);
        Ok(())
    }

    #[tokio::test]
    async fn bookmark_api_client_reads_bookmarks() -> Result<()> {
        let server: MockServer = MockServer::start();
        let bookmarks_path = "/bookmark";

        let hello_mock: Mock = server.mock(|when, then| {
            when.method(GET)
                .path(bookmarks_path);
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"data":[{"id":836,"url":"https://great.example/","title":"Example title","description":"Website Description","lastmodified":1662500203,"added":1662500203,"clickcount":0,"lastPreview":0,"available":true,"archivedFile":null,"userId":"dan","tags":["go"],"folders":[-1],"textContent":null,"htmlContent":null}],"status":"success"}"#);
        });

        let client = get_api_client(&server)?;
        let query_tags = vec![];
        let filters = vec![];
        let unavailable = false;
        let bookmarks = client
            .read_bookmarks(query_tags, filters, unavailable)
            .await?;
        hello_mock.assert();
        assert!(bookmarks.len() == 1);
        assert!(bookmarks[0].id == 836);
        assert!(bookmarks[0].url == "https://great.example/");
        assert!(bookmarks[0].description == "Website Description");
        assert!(bookmarks[0].tags.len() == 1);
        assert!(bookmarks[0].tags[0] == "go");
        Ok(())
    }
}
