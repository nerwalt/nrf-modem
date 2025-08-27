MEMORY
{
    /* Trusted Firmware-M (TFM) is flashed at the start */
    /* NOTE: These sizes are for a particular build of the TFM. Other builds (e.g. ones with 
    different features) might rerquire different section sizes. */
    FLASH                             : ORIGIN = 0x00008000, LENGTH = 0xf8000
    SECURE_RAM                        : ORIGIN = 0x20000000, LENGTH = 0x8000
    MODEM_RAM                         : ORIGIN = 0x20008000, LENGTH = 0x4568
    RAM                         (rwx) : ORIGIN = 0x2000C568, LENGTH = 0x33a98
}
