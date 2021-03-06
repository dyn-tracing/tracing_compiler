use assert_cmd::prelude::*; // Add methods on commands
use diffy;
use std::fs;
use std::path::Path; // Directory management
use std::process::Command; // Run programs
use test_case::test_case; // Parametrized tests

#[test_case("get_service_name.cql", vec![]; "get_service_name")]
#[test_case("height.cql", vec!["height.rs"]; "height")]
#[test_case("height_avg.cql", vec!["height.rs", "avg.rs"]; "height_avg")]
#[test_case("histogram.cql", vec!["histogram.rs"]; "inconclusive - histogram")]
#[test_case("request_size.cql", vec![]; "request_size")]
#[test_case("request_size_avg.cql", vec!["avg.rs"]; "request_size_avg")]
#[test_case("request_size_avg_trace_attr.cql", vec!["avg.rs"]; "request_size_avg_trace_attr")]
#[test_case("request_time.cql", vec![]; "request_time")]
#[test_case("latency.cql", vec!["latency.rs"]; "inconclusive - latency")]
fn check_compilation_envoy(
    query_name: &str,
    udf_names: Vec<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Static folders
    let proj_dir = Path::new("");
    let query_dir = proj_dir.join("example_queries");
    let udf_dir = proj_dir.join("example_udfs");

    // The input query in the folder is provided as test case
    let query_file = query_dir.join(query_name);
    // This is the binary to compile a query
    let mut cmd = Command::new(proj_dir.join("target/debug/snicket"));
    // Assemble the args, first the input query
    let mut args = vec!["-q", query_file.to_str().unwrap()];
    // Append every udf that is provided
    if udf_names.len() > 0 {
        args.push("-u");
    }
    let mut udf_vec = Vec::new();
    for udf_name in &udf_names {
        udf_vec.push(udf_dir.join(udf_name).to_str().unwrap().to_string());
    }
    for udf in &udf_vec {
        args.push(udf);
    }
    let out_dir = query_dir.join("envoy");
    let mut out_file = out_dir.join(query_name);
    out_file.set_extension("rs");
    args.extend(vec!["-o", out_file.to_str().unwrap()]);
    args.extend(vec!["-c", "envoy"]);
    args.extend(vec!["--root-node", "productpage-v1"]);
    cmd.args(args);
    cmd.assert().success();

    let mut ref_file = out_dir.join(query_name);
    ref_file.set_extension("rs.ref");
    let out_file_str = fs::read_to_string(out_file).unwrap();
    let ref_file_str = fs::read_to_string(ref_file).unwrap();
    if out_file_str != ref_file_str {
        let diff = diffy::create_patch(&ref_file_str, &out_file_str);
        let diff_color = diffy::PatchFormatter::new().with_color();
        panic!(
            "Files differ in the following way:\n{}",
            diff_color.fmt_patch(&diff)
        );
    }
    Ok(())
}

#[test_case("get_service_name.cql", vec![]; "get_service_name")]
#[test_case("height.cql", vec!["height.rs"]; "height")]
#[test_case("height_avg.cql", vec!["height.rs", "avg.rs"]; "height_avg")]
#[test_case("histogram.cql", vec!["histogram.rs"]; "inconclusive - histogram")]
#[test_case("request_size.cql", vec![]; "request_size")]
#[test_case("request_size_avg.cql", vec!["avg.rs"]; "request_size_avg")]
#[test_case("request_size_avg_trace_attr.cql", vec!["avg.rs"]; "request_size_avg_trace_attr")]
#[test_case("request_time.cql", vec![]; "request_time")]
#[test_case("latency.cql", vec!["latency.rs"]; "inconclusive - latency")]
fn check_compilation_sim(
    query_name: &str,
    udf_names: Vec<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Static folders
    let proj_dir = Path::new("");
    let query_dir = proj_dir.join("example_queries");
    let udf_dir = proj_dir.join("example_udfs");

    // The input query in the folder is provided as test case
    let query_file = query_dir.join(query_name);
    // This is the binary to compile a query
    let mut cmd = Command::new(proj_dir.join("target/debug/snicket"));
    // Assemble the args, first the input query
    let mut args = vec!["-q", query_file.to_str().unwrap()];
    // Append every udf that is provided
    if udf_names.len() > 0 {
        args.push("-u");
    }
    let mut udf_vec = Vec::new();
    for udf_name in &udf_names {
        udf_vec.push(udf_dir.join(udf_name).to_str().unwrap().to_string());
    }
    for udf in &udf_vec {
        args.push(udf);
    }
    let out_dir = query_dir.join("sim");
    let mut out_file = out_dir.join(query_name);
    out_file.set_extension("rs");
    args.extend(vec!["-o", out_file.to_str().unwrap()]);
    args.extend(vec!["-c", "sim"]);
    args.extend(vec!["--root-node", "productpage-v1"]);
    cmd.args(args);
    cmd.assert().success();

    let mut ref_file = out_dir.join(query_name);
    ref_file.set_extension("rs.ref");
    let out_file_str = fs::read_to_string(out_file).unwrap();
    let ref_file_str = fs::read_to_string(ref_file).unwrap();
    if out_file_str != ref_file_str {
        let diff = diffy::create_patch(&ref_file_str, &out_file_str);
        let diff_color = diffy::PatchFormatter::new().with_color();
        panic!(
            "Files differ in the following way:\n{}",
            diff_color.fmt_patch(&diff)
        );
    }
    Ok(())
}
