# Audio Sync & Endless Loop Fixes

## Issues Fixed

### 1. **Race Condition: False "Finished" Detection During Buffering**
**Problem**: Sink reports `empty()` during initial buffering, causing premature track skip  
**Solution**: Added `buffering` state flag that tracks first ~1 second of audio before allowing "finished" detection

### 2. **Stream Timeout/Stuck Detection**
**Problem**: If network stalls or MP3 decoder gets stuck, audio plays endless silence  
**Solution**: Added 5-second timeout detection - if no samples received for 5 seconds, force stream end

### 3. **Memory Bloat from Unbounded Buffer**
**Problem**: `mp3_buffer` grows indefinitely, can cause RAM issues on long tracks  
**Solution**: Trim buffer to 1MB after reaching 2MB limit, keeping only recent data for safety

### 4. **Duplicate Frame Processing**
**Problem**: Every chunk re-decodes entire buffer from start, wasting CPU and causing sync drift  
**Solution**: Track `processed_bytes` offset, only decode NEW bytes in each chunk

### 5. **Sink Length Mismatch**
**Problem**: Position calculation can drift from actual audio due to re-decoded frames  
**Solution**: Send only new frames by decoding incrementally, ensuring 1:1 frame→sample mapping

## Technical Changes

### `StreamingSource` struct
```rust
buffering: bool,           // Prevent false finish during initial load
samples_received: usize,   // Count for buffering threshold
last_sample_time: Instant, // Timeout detection
```

### `Iterator::next()` improvements
- Track buffering state (mark complete after 44100 samples / ~1 second)
- Detect 5-second timeout → force finish
- Return `Some(0)` (silence) only while buffering or stream active
- Return `None` only when truly finished AND buffered

### Streaming functions
- **Incremental decoding**: `Mp3Decoder::new(&buffer[processed_bytes..])` instead of `&buffer[..]`
- **Buffer trimming**: Keep last 1MB when buffer exceeds 2MB
- **Frame tracking removed**: No more `frame_index` comparisons, send all decoded frames immediately

## Testing Recommendations

1. **Long sessions** - Play 10+ tracks in a row to verify no memory leak
2. **Poor network** - Test with throttled connection to verify timeout works
3. **Seek operations** - Jump around in tracks to verify buffer trimming
4. **Shuffle + repeat** - Ensure no stuck states or premature skips

## Expected Behavior

- **No more endless silence** - Stream times out after 5 seconds of stall
- **Accurate position tracking** - No sync drift from duplicate frames
- **Memory stable** - Buffer stays under 2MB during playback
- **Clean track transitions** - Buffering state prevents false "finished" triggers
