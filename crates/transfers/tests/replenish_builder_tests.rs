use alloy_primitives::{address, Address, Bytes, U256};
use alloy_sol_types::SolCall;
use circles_abis::{BaseGroup, HubV2};
use circles_transfers::{TransferBuilder, TransferError};
use circles_types::CirclesConfig;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

fn demo_config(rpc_url: &str) -> CirclesConfig {
    CirclesConfig {
        circles_rpc_url: rpc_url.into(),
        pathfinder_url: "".into(),
        profile_service_url: "".into(),
        v1_hub_address: Address::ZERO,
        v2_hub_address: address!("0x0000000000000000000000000000000000000001"),
        name_registry_address: Address::ZERO,
        base_group_mint_policy: Address::ZERO,
        standard_treasury: Address::ZERO,
        core_members_group_deployer: Address::ZERO,
        base_group_factory_address: Address::ZERO,
        lift_erc20_address: Address::ZERO,
        invitation_escrow_address: Address::ZERO,
        invitation_farm_address: Address::ZERO,
        referrals_module_address: Address::ZERO,
    }
}

struct MockRpcServer {
    url: String,
    addr: std::net::SocketAddr,
    running: Arc<AtomicBool>,
    requests: Arc<Mutex<Vec<Value>>>,
    handle: Option<JoinHandle<()>>,
}

impl MockRpcServer {
    fn spawn(handler: impl Fn(&Value) -> Value + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock rpc");
        listener
            .set_nonblocking(true)
            .expect("set nonblocking listener");
        let addr = listener.local_addr().expect("listener addr");
        let running = Arc::new(AtomicBool::new(true));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let running_thread = Arc::clone(&running);
        let requests_thread = Arc::clone(&requests);
        let handler = Arc::new(handler);
        let handle = thread::spawn(move || {
            while running_thread.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let body = read_http_body(&mut stream);
                        let request: Value =
                            serde_json::from_slice(&body).expect("parse rpc request");
                        requests_thread
                            .lock()
                            .expect("lock requests")
                            .push(request.clone());
                        let response = handler(&request);
                        write_http_response(&mut stream, &response);
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            url: format!("http://{addr}"),
            addr,
            running,
            requests,
            handle: Some(handle),
        }
    }

    fn url(&self) -> &str {
        &self.url
    }

    fn requests(&self) -> Vec<Value> {
        self.requests.lock().expect("lock requests").clone()
    }
}

impl Drop for MockRpcServer {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn read_http_body(stream: &mut TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("set read timeout");
    let cloned = stream.try_clone().expect("clone stream");
    let mut reader = BufReader::new(cloned);
    let mut content_length = 0usize;
    let mut line = String::new();

    loop {
        line.clear();
        reader.read_line(&mut line).expect("read header line");
        if line == "\r\n" {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(length) = lower.strip_prefix("content-length:") {
            content_length = length.trim().parse().expect("parse content length");
        }
    }

    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).expect("read body");
    body
}

fn write_http_response(stream: &mut TcpStream, body: &Value) {
    let bytes = serde_json::to_vec(body).expect("serialize response");
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        bytes.len()
    );
    stream
        .write_all(response.as_bytes())
        .expect("write response headers");
    stream.write_all(&bytes).expect("write response body");
    stream.flush().expect("flush response");
}

fn json_rpc_success(id: &Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.clone(),
        "result": result,
    })
}

fn selector_hex<T: SolCall>(call: T) -> String {
    let encoded = call.abi_encode();
    format!("0x{}", hex_bytes(&encoded[..4]))
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(nibble_to_hex(byte >> 4));
        out.push(nibble_to_hex(byte & 0x0f));
    }
    out
}

fn nibble_to_hex(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        _ => (b'a' + (nibble - 10)) as char,
    }
}

fn bool_result(value: bool) -> Value {
    let mut encoded = [0u8; 32];
    encoded[31] = u8::from(value);
    json!(format!("0x{}", hex_bytes(&encoded)))
}

fn u256_result(value: U256) -> Value {
    let mut encoded = [0u8; 32];
    value
        .to_be_bytes::<32>()
        .iter()
        .enumerate()
        .for_each(|(idx, byte)| {
            encoded[idx] = *byte;
        });
    json!(format!("0x{}", hex_bytes(&encoded)))
}

fn address_result(value: Address) -> Value {
    let mut encoded = [0u8; 32];
    encoded[12..].copy_from_slice(value.as_slice());
    json!(format!("0x{}", hex_bytes(&encoded)))
}

fn recorded_method_count(requests: &[Value], method: &str) -> usize {
    requests
        .iter()
        .filter(|request| request["method"].as_str() == Some(method))
        .count()
}

fn eth_call_data(request: &Value) -> &str {
    request["params"]
        .as_array()
        .and_then(|params| params.first())
        .and_then(|call| call.get("data").or_else(|| call.get("input")))
        .and_then(Value::as_str)
        .expect("eth_call data")
}

#[tokio::test]
async fn construct_replenish_returns_safe_transfer_when_unwrapped_balance_suffices() {
    let from = address!("0xde374ece6fa50e781e81aac78e811b33d16912c7");
    let receiver = address!("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let token = address!("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let erc1155_id = U256::from(42u64);
    let to_token_id_selector = selector_hex(HubV2::toTokenIdCall {
        _avatar: Address::ZERO,
    });
    let server = MockRpcServer::spawn(move |request| match request["method"].as_str().unwrap() {
        "circlesV2_getTokenBalances" => json_rpc_success(
            &request["id"],
            json!([{
                "tokenId": format!("{token:#x}"),
                "balance": U256::from(200u64),
                "staticAttoCircles": U256::from(200u64),
                "token_owner": format!("{token:#x}"),
            }]),
        ),
        "circles_getTokenInfoBatch" => json_rpc_success(
            &request["id"],
            json!([{
                "block_number": 0,
                "timestamp": 0,
                "transaction_index": 0,
                "log_index": 0,
                "transaction_hash": format!("{:#x}", alloy_primitives::TxHash::ZERO),
                "version": 2,
                "info_type": null,
                "token_type": "CrcV2_RegisterHuman",
                "token": format!("{token:#x}"),
                "token_owner": format!("{token:#x}"),
            }]),
        ),
        "eth_call" => {
            let data = eth_call_data(request);
            assert!(data.starts_with(&to_token_id_selector));
            json_rpc_success(&request["id"], u256_result(erc1155_id))
        }
        other => panic!("unexpected method {other}"),
    });

    let builder = TransferBuilder::new(demo_config(server.url())).expect("builder");
    let txs = builder
        .construct_replenish(from, token, U256::from(100u64), Some(receiver))
        .await
        .expect("construct replenish");

    assert_eq!(txs.len(), 1);
    let expected = HubV2::safeTransferFromCall {
        _from: from,
        _to: receiver,
        _id: erc1155_id,
        _value: U256::from(100u64),
        _data: Bytes::default(),
    };
    assert_eq!(txs[0].to, demo_config(server.url()).v2_hub_address);
    assert_eq!(txs[0].data, Bytes::from(expected.abi_encode()));
    assert_eq!(
        recorded_method_count(&server.requests(), "circlesV2_findPath"),
        0
    );
}

#[tokio::test]
async fn construct_replenish_prefers_local_unwraps_before_pathfinding() {
    let from = address!("0xde374ece6fa50e781e81aac78e811b33d16912c7");
    let token = address!("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let dem_wrapper = address!("0xcccccccccccccccccccccccccccccccccccccccc");
    let inf_wrapper = address!("0xdddddddddddddddddddddddddddddddddddddddd");
    let server = MockRpcServer::spawn(move |request| match request["method"].as_str().unwrap() {
        "circlesV2_getTokenBalances" => json_rpc_success(
            &request["id"],
            json!([
                {
                    "tokenId": format!("{token:#x}"),
                    "balance": U256::from(100u64),
                    "staticAttoCircles": U256::from(100u64),
                    "token_owner": format!("{token:#x}"),
                },
                {
                    "tokenId": format!("{dem_wrapper:#x}"),
                    "balance": U256::from(200u64),
                    "staticAttoCircles": U256::from(200u64),
                    "token_owner": format!("{token:#x}"),
                },
                {
                    "tokenId": format!("{inf_wrapper:#x}"),
                    "balance": U256::from(300u64),
                    "staticAttoCircles": U256::from(300u64),
                    "token_owner": format!("{token:#x}"),
                }
            ]),
        ),
        "circles_getTokenInfoBatch" => json_rpc_success(
            &request["id"],
            json!([
                {
                    "block_number": 0,
                    "timestamp": 0,
                    "transaction_index": 0,
                    "log_index": 0,
                    "transaction_hash": format!("{:#x}", alloy_primitives::TxHash::ZERO),
                    "version": 2,
                    "info_type": null,
                    "token_type": "CrcV2_RegisterHuman",
                    "token": format!("{token:#x}"),
                    "token_owner": format!("{token:#x}"),
                },
                {
                    "block_number": 0,
                    "timestamp": 0,
                    "transaction_index": 0,
                    "log_index": 0,
                    "transaction_hash": format!("{:#x}", alloy_primitives::TxHash::ZERO),
                    "version": 2,
                    "info_type": null,
                    "token_type": "CrcV2_ERC20WrapperDeployed_Demurraged",
                    "token": format!("{dem_wrapper:#x}"),
                    "token_owner": format!("{token:#x}"),
                },
                {
                    "block_number": 0,
                    "timestamp": 1700000000,
                    "transaction_index": 0,
                    "log_index": 0,
                    "transaction_hash": format!("{:#x}", alloy_primitives::TxHash::ZERO),
                    "version": 2,
                    "info_type": null,
                    "token_type": "CrcV2_ERC20WrapperDeployed_Inflationary",
                    "token": format!("{inf_wrapper:#x}"),
                    "token_owner": format!("{token:#x}"),
                }
            ]),
        ),
        other => panic!("unexpected method {other}"),
    });

    let builder = TransferBuilder::new(demo_config(server.url())).expect("builder");
    let txs = builder
        .construct_replenish(from, token, U256::from(500u64), None)
        .await
        .expect("construct replenish");

    assert_eq!(txs.len(), 2);
    assert_eq!(txs[0].to, dem_wrapper);
    assert_eq!(txs[1].to, inf_wrapper);
    assert_eq!(
        recorded_method_count(&server.requests(), "circlesV2_findPath"),
        0
    );
}

#[tokio::test]
async fn construct_replenish_adds_temporary_trust_around_pathfinding_flow() {
    let from = address!("0xde374ece6fa50e781e81aac78e811b33d16912c7");
    let receiver = address!("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let token = address!("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let unit = U256::from(1_000_000_000_000u64);
    let is_trusted_selector = selector_hex(HubV2::isTrustedCall {
        _truster: Address::ZERO,
        _trustee: Address::ZERO,
    });
    let server = MockRpcServer::spawn(move |request| match request["method"].as_str().unwrap() {
        "circlesV2_getTokenBalances" => json_rpc_success(
            &request["id"],
            json!([{
                "tokenId": format!("{token:#x}"),
                "balance": U256::from(10u64) * unit,
                "staticAttoCircles": U256::from(10u64) * unit,
                "token_owner": format!("{token:#x}"),
            }]),
        ),
        "circles_getTokenInfoBatch" => json_rpc_success(
            &request["id"],
            json!([{
                "block_number": 0,
                "timestamp": 0,
                "transaction_index": 0,
                "log_index": 0,
                "transaction_hash": format!("{:#x}", alloy_primitives::TxHash::ZERO),
                "version": 2,
                "info_type": null,
                "token_type": "CrcV2_RegisterHuman",
                "token": format!("{token:#x}"),
                "token_owner": format!("{token:#x}"),
            }]),
        ),
        "circlesV2_findPath" => json_rpc_success(
            &request["id"],
            json!({
                "maxFlow": U256::from(90u64) * unit,
                "transfers": [{
                    "from": format!("{from:#x}"),
                    "to": format!("{receiver:#x}"),
                    "tokenOwner": format!("{token:#x}"),
                    "value": U256::from(90u64) * unit,
                }]
            }),
        ),
        "eth_call" => {
            let data = eth_call_data(request);
            assert!(data.starts_with(&is_trusted_selector));
            json_rpc_success(&request["id"], bool_result(false))
        }
        other => panic!("unexpected method {other}"),
    });

    let builder = TransferBuilder::new(demo_config(server.url()))
        .expect("builder")
        .with_approval_check(false);
    let txs = builder
        .construct_replenish(from, token, U256::from(100u64) * unit, Some(receiver))
        .await
        .expect("construct replenish");

    assert_eq!(txs.len(), 4);
    assert_eq!(txs[0].to, demo_config(server.url()).v2_hub_address);
    assert_eq!(txs[1].to, demo_config(server.url()).v2_hub_address);
    assert_eq!(txs[2].to, demo_config(server.url()).v2_hub_address);
    assert_eq!(txs[3].to, demo_config(server.url()).v2_hub_address);

    let requests = server.requests();
    let path_request = requests
        .iter()
        .find(|request| request["method"].as_str() == Some("circlesV2_findPath"))
        .expect("path request");
    assert_eq!(
        path_request["params"][0]["SimulatedTrusts"],
        json!([{
            "truster": format!("{from:#x}"),
            "trustee": format!("{token:#x}"),
        }])
    );
}

#[tokio::test]
async fn construct_replenish_handles_wrapped_path_branch() {
    let from = address!("0xde374ece6fa50e781e81aac78e811b33d16912c7");
    let receiver = address!("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let token = address!("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let wrapper = address!("0xcccccccccccccccccccccccccccccccccccccccc");
    let unit = U256::from(1_000_000_000_000u64);
    let is_trusted_selector = selector_hex(HubV2::isTrustedCall {
        _truster: Address::ZERO,
        _trustee: Address::ZERO,
    });
    let server = MockRpcServer::spawn(move |request| match request["method"].as_str().unwrap() {
        "circlesV2_getTokenBalances" => json_rpc_success(&request["id"], json!([])),
        "circles_getTokenInfoBatch" => {
            let requested = request["params"][0]
                .as_array()
                .expect("token batch params")
                .iter()
                .map(|value| value.as_str().unwrap().to_lowercase())
                .collect::<Vec<_>>();
            assert!(requested.contains(&format!("{wrapper:#x}")));
            json_rpc_success(
                &request["id"],
                json!([{
                    "block_number": 0,
                    "timestamp": 0,
                    "transaction_index": 0,
                    "log_index": 0,
                    "transaction_hash": format!("{:#x}", alloy_primitives::TxHash::ZERO),
                    "version": 2,
                    "info_type": null,
                    "token_type": "CrcV2_ERC20WrapperDeployed_Demurraged",
                    "token": format!("{wrapper:#x}"),
                    "token_owner": format!("{token:#x}"),
                }]),
            )
        }
        "circlesV2_findPath" => json_rpc_success(
            &request["id"],
            json!({
                "maxFlow": U256::from(100u64) * unit,
                "transfers": [{
                    "from": format!("{from:#x}"),
                    "to": format!("{receiver:#x}"),
                    "tokenOwner": format!("{wrapper:#x}"),
                    "value": U256::from(100u64) * unit,
                }]
            }),
        ),
        "eth_call" => {
            let data = eth_call_data(request);
            assert!(data.starts_with(&is_trusted_selector));
            json_rpc_success(&request["id"], bool_result(true))
        }
        other => panic!("unexpected method {other}"),
    });

    let builder = TransferBuilder::new(demo_config(server.url()))
        .expect("builder")
        .with_approval_check(false);
    let txs = builder
        .construct_replenish(from, token, U256::from(100u64) * unit, Some(receiver))
        .await
        .expect("construct replenish");

    assert_eq!(txs.len(), 3);
    assert_eq!(txs[0].to, demo_config(server.url()).v2_hub_address);
    assert_eq!(txs[1].to, wrapper);
    assert_eq!(txs[2].to, demo_config(server.url()).v2_hub_address);
    let requests = server.requests();
    let path_request = requests
        .iter()
        .find(|request| request["method"].as_str() == Some("circlesV2_findPath"))
        .expect("path request");
    assert_eq!(path_request["params"][0]["SimulatedTrusts"], Value::Null);
}

#[tokio::test]
async fn construct_group_token_redeem_matches_ts_flow_shape() {
    let from = address!("0xde374ece6fa50e781e81aac78e811b33d16912c7");
    let group = address!("0x1111111111111111111111111111111111111111");
    let treasury = address!("0x2222222222222222222222222222222222222222");
    let collateral_a = address!("0x3333333333333333333333333333333333333333");
    let collateral_b = address!("0x4444444444444444444444444444444444444444");
    let amount = U256::from(100u64);
    let treasury_selector = selector_hex(BaseGroup::BASE_TREASURYCall {});

    let server = MockRpcServer::spawn(move |request| match request["method"].as_str().unwrap() {
        "circles_getTokenInfo" => json_rpc_success(
            &request["id"],
            json!({
                "block_number": 0,
                "timestamp": 0,
                "transaction_index": 0,
                "log_index": 0,
                "transaction_hash": format!("{:#x}", alloy_primitives::TxHash::ZERO),
                "version": 2,
                "info_type": null,
                "token_type": "CrcV2_RegisterGroup",
                "token": format!("{group:#x}"),
                "token_owner": format!("{group:#x}"),
            }),
        ),
        "circlesV2_getTokenBalances" => json_rpc_success(
            &request["id"],
            json!([
                {
                    "tokenAddress": format!("{collateral_a:#x}"),
                    "tokenId": format!("{collateral_a:#x}"),
                    "tokenOwner": format!("{collateral_a:#x}"),
                    "tokenType": "CrcV2_RegisterHuman",
                    "version": 2,
                    "attoCircles": "1000",
                    "circles": 0.000000000000001,
                    "staticAttoCircles": "1000",
                    "staticCircles": 0.000000000000001,
                    "attoCrc": "1000",
                    "crc": 0.000000000000001,
                    "isErc20": false,
                    "isErc1155": true,
                    "isWrapped": false,
                    "isInflationary": false,
                    "isGroup": false
                },
                {
                    "tokenAddress": format!("{collateral_b:#x}"),
                    "tokenId": format!("{collateral_b:#x}"),
                    "tokenOwner": format!("{collateral_b:#x}"),
                    "tokenType": "CrcV2_RegisterHuman",
                    "version": 2,
                    "attoCircles": "1000",
                    "circles": 0.000000000000001,
                    "staticAttoCircles": "1000",
                    "staticCircles": 0.000000000000001,
                    "attoCrc": "1000",
                    "crc": 0.000000000000001,
                    "isErc20": false,
                    "isErc1155": true,
                    "isWrapped": false,
                    "isInflationary": false,
                    "isGroup": false
                }
            ]),
        ),
        "circles_getAggregatedTrustRelations" => json_rpc_success(
            &request["id"],
            json!([
                {
                    "subject_avatar": format!("{from:#x}"),
                    "relation": "trusts",
                    "object_avatar": format!("{collateral_a:#x}"),
                    "timestamp": 0
                },
                {
                    "subject_avatar": format!("{from:#x}"),
                    "relation": "mutuallyTrusts",
                    "object_avatar": format!("{collateral_b:#x}"),
                    "timestamp": 0
                }
            ]),
        ),
        "circlesV2_findPath" => json_rpc_success(
            &request["id"],
            json!({
                "maxFlow": amount,
                "transfers": [{
                    "from": format!("{from:#x}"),
                    "to": format!("{from:#x}"),
                    "tokenOwner": format!("{collateral_a:#x}"),
                    "value": amount,
                }]
            }),
        ),
        "circles_getTokenInfoBatch" => json_rpc_success(
            &request["id"],
            json!([{
                "block_number": 0,
                "timestamp": 0,
                "transaction_index": 0,
                "log_index": 0,
                "transaction_hash": format!("{:#x}", alloy_primitives::TxHash::ZERO),
                "version": 2,
                "info_type": null,
                "token_type": "CrcV2_RegisterHuman",
                "token": format!("{collateral_a:#x}"),
                "token_owner": format!("{collateral_a:#x}"),
            }]),
        ),
        "eth_call" => {
            let data = eth_call_data(request);
            assert!(data.starts_with(&treasury_selector));
            json_rpc_success(&request["id"], address_result(treasury))
        }
        other => panic!("unexpected method {other}"),
    });

    let builder = TransferBuilder::new(demo_config(server.url()))
        .expect("builder")
        .with_approval_check(false);
    let txs = builder
        .construct_group_token_redeem(from, group, amount)
        .await
        .expect("construct group token redeem");

    assert_eq!(txs.len(), 2);
    assert_eq!(txs[0].to, demo_config(server.url()).v2_hub_address);
    assert_eq!(txs[1].to, demo_config(server.url()).v2_hub_address);
    assert_eq!(
        recorded_method_count(&server.requests(), "circlesV2_findPath"),
        2
    );

    let first_path_request = server
        .requests()
        .into_iter()
        .find(|request| request["method"].as_str() == Some("circlesV2_findPath"))
        .expect("first path request");
    assert_eq!(
        first_path_request["params"][0]["UseWrappedBalances"],
        json!(false)
    );
    assert_eq!(
        first_path_request["params"][0]["FromTokens"],
        json!([format!("{group:#x}")])
    );
    assert_eq!(
        first_path_request["params"][0]["ToTokens"],
        json!([format!("{collateral_a:#x}"), format!("{collateral_b:#x}")])
    );
}

#[tokio::test]
async fn construct_group_token_redeem_fails_without_trusted_collateral() {
    let from = address!("0xde374ece6fa50e781e81aac78e811b33d16912c7");
    let group = address!("0x1111111111111111111111111111111111111111");
    let treasury = address!("0x2222222222222222222222222222222222222222");
    let collateral = address!("0x3333333333333333333333333333333333333333");
    let treasury_selector = selector_hex(BaseGroup::BASE_TREASURYCall {});

    let server = MockRpcServer::spawn(move |request| match request["method"].as_str().unwrap() {
        "circles_getTokenInfo" => json_rpc_success(
            &request["id"],
            json!({
                "block_number": 0,
                "timestamp": 0,
                "transaction_index": 0,
                "log_index": 0,
                "transaction_hash": format!("{:#x}", alloy_primitives::TxHash::ZERO),
                "version": 2,
                "info_type": null,
                "token_type": "CrcV2_RegisterGroup",
                "token": format!("{group:#x}"),
                "token_owner": format!("{group:#x}"),
            }),
        ),
        "circlesV2_getTokenBalances" => json_rpc_success(
            &request["id"],
            json!([{
                "tokenAddress": format!("{collateral:#x}"),
                "tokenId": format!("{collateral:#x}"),
                "tokenOwner": format!("{collateral:#x}"),
                "tokenType": "CrcV2_RegisterHuman",
                "version": 2,
                "attoCircles": "1000",
                "circles": 0.000000000000001,
                "staticAttoCircles": "1000",
                "staticCircles": 0.000000000000001,
                "attoCrc": "1000",
                "crc": 0.000000000000001,
                "isErc20": false,
                "isErc1155": true,
                "isWrapped": false,
                "isInflationary": false,
                "isGroup": false
            }]),
        ),
        "circles_getAggregatedTrustRelations" => json_rpc_success(&request["id"], json!([])),
        "eth_call" => {
            let data = eth_call_data(request);
            assert!(data.starts_with(&treasury_selector));
            json_rpc_success(&request["id"], address_result(treasury))
        }
        other => panic!("unexpected method {other}"),
    });

    let builder = TransferBuilder::new(demo_config(server.url())).expect("builder");
    let err = builder
        .construct_group_token_redeem(from, group, U256::from(10u64))
        .await
        .expect_err("missing trusted collateral should fail");

    match err {
        TransferError::Generic { code, .. } => {
            assert_eq!(
                code.as_deref(),
                Some("GROUP_TOKEN_REDEEM_NO_TRUSTED_COLLATERAL")
            );
        }
        other => panic!("unexpected error: {other:?}"),
    }
    assert_eq!(
        recorded_method_count(&server.requests(), "circlesV2_findPath"),
        0
    );
}
