use std::convert::Infallible;
use std::net::SocketAddr;
use std::str;

use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::Frame;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Body, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use fast_qr::convert::ConvertError;
use fast_qr::convert::{image::ImageBuilder, svg::SvgBuilder, Builder, Shape};
use fast_qr::qr::QRBuilder;

async fn qr(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        // Serve some instructions at /
        (&Method::GET, "/") => Ok(Response::new(full(
            "Try POSTing data to /build such as: `curl localhost:3000/build -XPOST -d \"Hello World!\"`",
        ))),

        (&Method::POST, "/build") => {
            let max = req.body().size_hint().upper().unwrap_or(u64::MAX);
            if max > 1024 * 64 {
                let mut resp = Response::new(full("Body too big"));
                *resp.status_mut() = hyper::StatusCode::PAYLOAD_TOO_LARGE;
                return Ok(resp);
            }

            let whole_body = req.collect().await?.to_bytes();

            let qrcode = QRBuilder::new(str::from_utf8(&whole_body).unwrap())
                .build()
                .unwrap();

            Ok(Response::new(full(qrcode.to_str())))
        }

        (&Method::POST, "/build/svg") => {
            let max = req.body().size_hint().upper().unwrap_or(u64::MAX);
            if max > 1024 * 64 {
                let mut resp = Response::new(full("Body too big"));
                *resp.status_mut() = hyper::StatusCode::PAYLOAD_TOO_LARGE;
                return Ok(resp);
            }

            let whole_body = req.collect().await?.to_bytes();

            let qrcode = QRBuilder::new(str::from_utf8(&whole_body).unwrap())
                .build()
                .unwrap();

            let _svg = SvgBuilder::default()
                .shape(Shape::RoundedSquare)
                .to_str(&qrcode);

            Ok(Response::new(full(_svg)))
        }

        (&Method::POST, "/build/png") => {
            let max = req.body().size_hint().upper().unwrap_or(u64::MAX);
            if max > 1024 * 64 {
                let mut resp = Response::new(full("Body too big"));
                *resp.status_mut() = hyper::StatusCode::PAYLOAD_TOO_LARGE;
                return Ok(resp);
            }

            let whole_body = req.collect().await?.to_bytes();

            let qrcode = QRBuilder::new(str::from_utf8(&whole_body).unwrap())
                .build()
                .unwrap();

            let _img = ImageBuilder::default()
                .shape(Shape::RoundedSquare)
                .fit_width(600)
                .to_bytes(&qrcode);

            Ok(Response::new(full(_img.expect())))
        }

        // Return the 404 Not Found for other routes.
        _ => {
            let mut not_found = Response::new(empty());
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(qr))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
