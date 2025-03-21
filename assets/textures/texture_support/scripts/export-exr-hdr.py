# python ./scripts/export-exr-hdr.py

import numpy as np
import OpenEXR
import Imath
import imageio.v3 as iio

def srgb_to_linear(srgb):
    """ Convert sRGB to linear using the standard formula. """
    return np.where(srgb <= 0.04045,
                    srgb / 12.92,
                    ((srgb + 0.055) / 1.055) ** 2.4)

# Load PNG image (sRGB)
input_filename = "png-srgb-rgb.png"
img_srgb = iio.imread(input_filename).astype(np.float32) / 255.0  # Normalize to [0,1]

# Convert sRGB to linear space
img_linear = srgb_to_linear(img_srgb)

# Identify pure-white pixels in sRGB space (R, G, B all == 1.0 in sRGB)
pure_white_mask = np.all(img_srgb == 1.0, axis=-1)  # True where pixels are exactly (1,1,1) in sRGB

# Apply boost only to pure-white pixels in linear space
img_linear[pure_white_mask] *= 10.0  # Multiply only the pure white pixels

# Ensure shape is (3, height, width) for OpenEXR
img_exr = img_linear.transpose(2, 0, 1)

# Define EXR output
height, width = img_exr.shape[1], img_exr.shape[2]
output_exr_filename = "exr-hdr.exr"
header = OpenEXR.Header(width, height)
channels = ["R", "G", "B"]
pixel_type = Imath.PixelType(Imath.PixelType.FLOAT)

# Prepare the image data for OpenEXR
exr_file = OpenEXR.OutputFile(output_exr_filename, header)
exr_file.writePixels({ch: img_exr[i].astype(np.float32).tobytes() for i, ch in enumerate(channels)})
print(f"Saved linear HDR image to {output_exr_filename}")
