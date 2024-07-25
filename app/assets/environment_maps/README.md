# Environment maps

The `pisa_*.ktx2` files were generated from [pisa](https://github.com/KhronosGroup/glTF-Sample-Environments/blob/master/pisa.hdr).

## [glTF-IBL-Sampler](https://github.com/KhronosGroup/glTF-IBL-Sampler)

For IBL environment map prefiltering to cubemaps:

```bash
./cli -inputPath pisa.hdr -outCubeMap pisa_diffuse.ktx2 -distribution Lambertian -cubeMapResolution 32
./cli -inputPath pisa.hdr -outCubeMap pisa_specular.ktx2 -distribution GGX -cubeMapResolution 512
```

## [bevy_mod_environment_map_tools](https://github.com/DGriffin91/bevy_mod_environment_map_tools)

For converting to rgb9e5 format with zstd 'supercompression':

```bash
cargo run --release -- --inputs pisa_diffuse.ktx2,pisa_specular.ktx2 --outputs pisa_diffuse_rgb9e5_zstd.ktx2,pisa_specular_rgb9e5_zstd.ktx2
```
