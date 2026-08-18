```
Usage: equirect2cubemap.exe [-f <format>] [-i <interpolation>] [-s <size>] [-r] [-t] [-e <exposure>] [-u] [--] <input> <output>

Configuration of the conversion.

Positional Arguments:
  input             the input equirectangular image's path
  output            the directory to put the output images in, creates if not
                    exists

Options:
  -f, --format      the format of the output images [possible values: jpg, png,
                    webp, hdr, exr]
  -i, --interpolation
                    interpolation used when sampling source image [possible
                    values: linear, nearest]
  -s, --size        size (px) of the output images, width = height
  -r, --rotate      rotate to a z-up skybox if you use it in a y-up renderer
  -t, --tone-mapping
                    enable tone mapping (Reinhard)
  -e, --exposure    exposure of tone mapping
  -u, --unlimited   remove the limits on image size and memory usage (could
                    cause OOM on large images or decompression bombs)
  --help, help      display usage information
```
