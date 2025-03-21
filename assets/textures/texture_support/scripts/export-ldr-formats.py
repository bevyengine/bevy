# python ./scripts/export-ldr-formats.py

from PIL import Image
import imageio.v3 as iio
import numpy as np
import qoi
import os

# Load the original PNG image
input_filename = "png-srgb-rgb.png"
img = Image.open(input_filename)

# Convert the image to a NumPy array for formats that require it
img_array = np.array(img)

# List of formats to convert to
formats = {
    "bmp": "bmp",
    "dds": "dds",
    "gif": "gif",
    "ico": "ico",
    "jpg": "jpeg",
    "qoi": "qoi",
    "tga": "tga",
    "tif": "tiff",
    "webp": "webp",
    "pam": "pam",
    "ppm": "ppm"
}

# Create output directory
output_dir = "./"

for ext, fmt in formats.items():
    if fmt is None:
        print(f"Skipping {ext} (unsupported format)")
        continue

    output_filename = os.path.join(output_dir, f"{ext}.{ext}")

    try:
        if fmt == "qoi":
            # Convert to RGBA as required for QOI
            img_rgba = img.convert("RGBA")
            img_data = np.array(img_rgba)
            with open(output_filename, "wb") as f:
                f.write(qoi.encode(img_data))
        elif fmt == "dds":
            # Convert to BGR
            img_rgba = img.convert("RGB")
            img_array_rgba = np.array(img_rgba)
            img_bgra = img_array_rgba[..., [2, 1, 0]] # Swap R and B
            iio.imwrite(output_filename, img_bgra, extension=".dds")
        elif fmt == "pam":
            iio.imwrite(output_filename, img_array)
        elif fmt == "ico":
            # Convert to RGBA explicitly
            img_rgba = img.convert("RGBA")
            img_rgba.save(output_filename, format="ICO")
        else:
            img.save(output_filename, format=fmt.upper())

        print(f"Saved: {output_filename}")
    except Exception as e:
        print(f"Failed to save {ext}: {e}")
