     STDIN
   1 #ifndef SCRIBE_H
   2 #define SCRIBE_H
   3 
   4 #include <stdint.h>
   5 
   6 #ifdef __cplusplus
   7 extern "C" {
   8 #endif
   9 
  10 /**
  11  * Initialize the Scribe logging system.
  12  * 
  13  * @param config_path Path to the configuration file (can be NULL for defaults)
  14  * @return 0 on success, negative value on error
  15  */
  16 int scribe_init(const char* config_path);
  17 
  18 /**
  19  * Log a message.
  20  * 
  21  * @param level Log level (0=Verbose, 1=Debug, 2=Info, 3=Warn, 4=Error)
  22  * @param label Tag/label for the log message
  23  * @param message The log message
  24  * @return 0 on success, negative value on error
  25  */
  26 int scribe_log(int level, const char* label, const char* message);
  27 
  28 /**
  29  * Flush all pending log data to disk.
  30  * 
  31  * @return 0 on success, negative value on error
  32  */
  33 int scribe_flush(void);
  34 
  35 /**
  36  * Performance statistics structure.
  37  */
  38 typedef struct {
  39     uint64_t total_writes;
  40     uint64_t total_flushes;
  41     uint64_t total_errors;
  42     uint64_t bytes_written;
  43     uint64_t last_cleanup_time;
  44 } ScribeStats;
  45 
  46 /**
  47  * Get performance statistics.
  48  * 
  49  * @param stats Pointer to ScribeStats structure to fill
  50  * @return 0 on success, negative value on error
  51  */
  52 int scribe_get_stats(ScribeStats* stats);
  53 
  54 #ifdef __cplusplus
  55 }
  56 #endif
  57 
  58 #endif // SCRIBE_H
