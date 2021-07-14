use structopt::StructOpt;

use crate::http::{guess_content_type, ContentType, HttpMethod};

#[derive(Debug, Clone, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct CliArgs {
    #[structopt(
        short = "m",
        long,
        default_value = "GET",
        help = "The HTTP method to use (case-insensitive). \
        Supported methods: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS"
    )]
    pub method: HttpMethod,
    #[structopt(
        short = "t",
        long = "type",
        help = "Value for the Content-Type header, can be:\n\
                - text: for text/plain\n\
                - json: for application/json\n\
                - form: for application/x-www-form-urlencoded\n\
                - multipart: for multipart/form-data\n\
                By default the content type will be guessed based on the request body,
                but this guess may not be correct, so specifying the content type explicitly \
                is recommended."
    )]
    pub content_type: Option<ContentType>,
    #[structopt(short, long, help = "The request body")]
    pub data: Option<String>,
    #[structopt(help = "The URL to send the request to")]
    pub url: String,
}

/// Parse the command line arguments
pub fn args() -> CliArgs {
    let mut args = CliArgs::from_args();
    // Guess content type if not provided
    if let Some(body) = &args.data {
        if args.content_type.is_none() {
            args.content_type = Some(guess_content_type(body));
        }
    }
    let url = args.url.as_str();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        args.url.insert_str(0, "http://");
    }
    args
}
