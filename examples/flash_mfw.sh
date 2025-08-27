MFW_ZIP=$1
nrfjprog --family NRF91 --chiperase --verify --program $MFW_ZIP
