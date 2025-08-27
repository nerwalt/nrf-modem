# spm.hex is for the nRF9160
nrfjprog --family NRF91 --recover
nrfjprog --family NRF91 --chiperase --verify --program ./spm.hex
