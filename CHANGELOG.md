0.4
- added more interpolation methods: lanczos, mitchell, catmull-rom
- the output image size no longer defaults to 512, it's now computed from the input size (1/4 of the input image width) if not specified. You can use the `--pot` option to force a power-of-two size.
