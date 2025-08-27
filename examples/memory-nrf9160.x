MEMORY
{
  /* Secure Partition Manager (SPM) is flashed at the start */
  /* NOTE: 1 K = 1 KiBi = 1024 bytes */
  FLASH                             : ORIGIN = 0x00050000, LENGTH = 512K
  RAM                         (rwx) : ORIGIN = 0x20018000, LENGTH = 160K
}
