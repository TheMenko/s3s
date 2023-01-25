use crate::dto::RustTypes;
use crate::f;
use crate::gen::Codegen;
use crate::ops::Operations;
use crate::rust;

use heck::ToSnakeCase;

pub fn codegen(ops: &Operations, rust_types: &RustTypes, g: &mut Codegen) {
    g.ln("use super::*;");
    g.lf();
    g.ln("use crate::conv::{try_from_aws, try_into_aws};");
    g.lf();
    g.ln("use s3s::S3;");
    g.ln("use s3s::S3Result;");
    g.lf();
    g.ln("use tracing::debug;");
    g.lf();

    g.ln("#[async_trait::async_trait]");
    g.ln("impl S3 for Proxy {");

    for op in ops.values() {
        let method_name = op.name.to_snake_case();
        let s3s_input = f!("s3s::dto::{}", op.input);
        let s3s_output = f!("s3s::dto::{}", op.output);

        g.ln("#[tracing::instrument(skip(self, input))]");
        g.ln(f!("async fn {method_name}(&self, input: {s3s_input}) -> S3Result<{s3s_output}> {{"));

        g.ln("debug!(?input);");

        if op.smithy_input == "Unit" {
            g.ln(f!("let result = self.0.{method_name}().send().await;"));
        } else {
            g.ln(f!("let mut b = self.0.{method_name}();"));
            let rust::Type::Struct(ty) = &rust_types[op.input.as_str()] else { panic!() };
            for field in &ty.fields {
                let s3s_field_name = field.name.as_str();
                let aws_field_name = match s3s_field_name {
                    "checksum_crc32c" => "checksum_crc32_c",
                    "type_" => "type",
                    s => s,
                };

                if field.type_ == "StreamingBlob" {
                    g.ln(f!("b = b.set_{aws_field_name}(Some(transform_body(input.{s3s_field_name}).await));"));
                } else if field.option_type {
                    g.ln(f!("b = b.set_{aws_field_name}(try_into_aws(input.{s3s_field_name})?);"));
                } else {
                    g.ln(f!("b = b.set_{aws_field_name}(Some(try_into_aws(input.{s3s_field_name})?));"));
                }
            }
            g.ln("let result = b.send().await;");
        }

        g.ln("match result {");
        g.ln("Ok(output) => {");
        g.ln("    let output = try_from_aws(output)?;");
        g.ln("    debug!(?output);");
        g.ln("    Ok(output)");
        g.ln("},");
        g.ln("Err(e) => Err(wrap_sdk_error!(e)),");
        g.ln("}");

        g.ln("}");
        g.lf();
    }

    g.ln("}");
}