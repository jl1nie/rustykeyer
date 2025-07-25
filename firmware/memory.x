/* CH32V203 Memory Layout */
MEMORY
{
  /* CH32V203C8T6 has 64K flash and 20K RAM */
  FLASH : ORIGIN = 0x08000000, LENGTH = 64K
  RAM : ORIGIN = 0x20000000, LENGTH = 20K
}

/* Define regions for sections */
REGION_ALIAS("REGION_TEXT", FLASH);
REGION_ALIAS("REGION_RODATA", FLASH);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RAM);
REGION_ALIAS("REGION_STACK", RAM);

/* Stack size - 2KB (minimal for Embassy) */
_hart_stack_size = 2K;
_max_hart_id = 0;