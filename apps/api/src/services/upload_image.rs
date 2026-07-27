use std::io::Cursor;

use bytes::Bytes;
use image::{imageops::FilterType, DynamicImage, ImageFormat, ImageReader};

use crate::models::UploadCategory;

pub struct ProcessedImage {
    pub data: Bytes,
    pub thumbnail: Bytes,
    pub mime_type: &'static str,
    pub extension: &'static str,
    pub width: i32,
    pub height: i32,
}

pub fn process_image(
    category: UploadCategory,
    data: Bytes,
    mime_type: &str,
) -> anyhow::Result<ProcessedImage> {
    let format = format_for_mime(mime_type).ok_or_else(|| anyhow::anyhow!("unsupported image"))?;
    let dimensions = ImageReader::new(Cursor::new(data.as_ref()))
        .with_guessed_format()?
        .into_dimensions()?;
    let pixels = u64::from(dimensions.0) * u64::from(dimensions.1);
    if dimensions.0 == 0 || dimensions.1 == 0 || pixels > 40_000_000 {
        anyhow::bail!("image dimensions are not allowed");
    }

    let decoded = image::load_from_memory_with_format(&data, format)?;
    let processed = match category {
        UploadCategory::Avatar => {
            let side = decoded.width().min(decoded.height()).min(512);
            decoded.resize_to_fill(side, side, FilterType::Lanczos3)
        }
        UploadCategory::TopicImage if decoded.width() > 2560 || decoded.height() > 2560 => {
            decoded.resize(2560, 2560, FilterType::Lanczos3)
        }
        UploadCategory::CommentImage if decoded.width() > 1920 || decoded.height() > 1920 => {
            decoded.resize(1920, 1920, FilterType::Lanczos3)
        }
        UploadCategory::TopicImage | UploadCategory::CommentImage => decoded,
        UploadCategory::Attachment => anyhow::bail!("attachment is not an image category"),
    };
    let thumbnail = match category {
        UploadCategory::Avatar => processed.resize_to_fill(128, 128, FilterType::Lanczos3),
        _ => processed.thumbnail(480, 480),
    };

    let main = if format == ImageFormat::Gif {
        data
    } else {
        encode(&processed, format)?
    };
    let thumbnail = encode(&thumbnail, ImageFormat::Png)?;

    Ok(ProcessedImage {
        data: main,
        thumbnail,
        mime_type: mime_for_format(format),
        extension: extension_for_format(format),
        width: i32::try_from(processed.width())?,
        height: i32::try_from(processed.height())?,
    })
}

fn encode(image: &DynamicImage, format: ImageFormat) -> anyhow::Result<Bytes> {
    let mut output = Cursor::new(Vec::new());
    image.write_to(&mut output, format)?;
    Ok(Bytes::from(output.into_inner()))
}

fn format_for_mime(mime_type: &str) -> Option<ImageFormat> {
    match mime_type {
        "image/jpeg" => Some(ImageFormat::Jpeg),
        "image/png" => Some(ImageFormat::Png),
        "image/webp" => Some(ImageFormat::WebP),
        "image/gif" => Some(ImageFormat::Gif),
        _ => None,
    }
}

fn mime_for_format(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::Png => "image/png",
        ImageFormat::WebP => "image/webp",
        ImageFormat::Gif => "image/gif",
        _ => unreachable!("format is restricted by format_for_mime"),
    }
}

fn extension_for_format(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Jpeg => "jpg",
        ImageFormat::Png => "png",
        ImageFormat::WebP => "webp",
        ImageFormat::Gif => "gif",
        _ => unreachable!("format is restricted by format_for_mime"),
    }
}
