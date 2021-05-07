use chrono::{DateTime, Utc};
use disk::agg::scalarbinbatch::MinMaxAvgScalarBinBatchStreamItem;
use disk::cache::CacheUsage;
use disk::frame::inmem::InMemoryFrameAsyncReadStream;
use disk::streamlog::Streamlog;
use err::Error;
use futures_util::TryStreamExt;
use http::StatusCode;
use hyper::Body;
use netpod::log::*;
use netpod::PerfOpts;

pub async fn get_binned(
    host: String,
    port: u16,
    channel_backend: String,
    channel_name: String,
    beg_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    bin_count: u32,
    cache_usage: CacheUsage,
) -> Result<(), Error> {
    info!("-------   get_binned  client");
    let t1 = Utc::now();
    let date_fmt = "%Y-%m-%dT%H:%M:%S.%3fZ";
    let uri = format!(
        "http://{}:{}/api/1/binned?channel_backend={}&channel_name={}&beg_date={}&end_date={}&bin_count={}&cache_usage={}",
        host,
        port,
        channel_backend,
        channel_name,
        beg_date.format(date_fmt),
        end_date.format(date_fmt),
        bin_count,
        cache_usage.query_param_value(),
    );
    info!("get_binned  uri {:?}", uri);
    let req = hyper::Request::builder()
        .method(http::Method::GET)
        .uri(uri)
        .body(Body::empty())?;
    let client = hyper::Client::new();
    let res = client.request(req).await?;
    if res.status() != StatusCode::OK {
        error!("Server error  {:?}", res);
        return Err(Error::with_msg(format!("Server error  {:?}", res)));
    }
    let perf_opts = PerfOpts { inmem_bufcap: 512 };
    let s1 = disk::cache::HttpBodyAsAsyncRead::new(res);
    let s2 = InMemoryFrameAsyncReadStream::new(s1, perf_opts.inmem_bufcap);
    use futures_util::StreamExt;
    use std::future::ready;
    let mut bin_count = 0;
    let s3 = s2
        .map_err(|e| error!("get_binned  {:?}", e))
        .filter_map(|item| {
            let g = match item {
                Ok(frame) => {
                    type ExpectedType = disk::cache::BinnedBytesForHttpStreamFrame;
                    let n1 = frame.buf().len();
                    match bincode::deserialize::<ExpectedType>(frame.buf()) {
                        Ok(item) => match item {
                            Ok(item) => {
                                match &item {
                                    MinMaxAvgScalarBinBatchStreamItem::Log(item) => {
                                        Streamlog::emit(item);
                                    }
                                    MinMaxAvgScalarBinBatchStreamItem::Values(item) => {
                                        bin_count += 1;
                                        info!("len {}  values {:?}", n1, item);
                                    }
                                    item => {
                                        info!("len {}  item {:?}", n1, item);
                                    }
                                }
                                Some(Ok(item))
                            }
                            Err(e) => {
                                error!("len {}  error frame {:?}", n1, e);
                                Some(Err(e))
                            }
                        },
                        Err(e) => {
                            error!("len {}  bincode error {:?}", n1, e);
                            Some(Err(e.into()))
                        }
                    }
                }
                Err(e) => Some(Err(Error::with_msg(format!("{:?}", e)))),
            };
            ready(g)
        })
        .for_each(|_| ready(()));
    s3.await;
    let t2 = chrono::Utc::now();
    let ntot = 0;
    let ms = t2.signed_duration_since(t1).num_milliseconds() as u64;
    let throughput = ntot / 1024 * 1000 / ms;
    info!(
        "get_cached_0 DONE  total download {} MB   throughput {:5} kB/s  bin_count {}",
        ntot / 1024 / 1024,
        throughput,
        bin_count,
    );
    Ok(())
}
