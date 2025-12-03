# 🐞 Bug Report – SoundCloud Related Tracks API Returning 404
**Date:** 2025-12-01  
**System:** TempRS Audio Player / Home Data Pipeline  
**Component:** `TempRS::api::tracks → fetch_related()`  
**Severity:** ⚠️ Medium

---

## Summary
The TempRS music player attempted to fetch related tracks from the SoundCloud API.  
The request failed with: **404 Not Found**, causing Home recommendations to be empty.

---

## Logs
```
[2025-12-01T22:09:26Z INFO  TempRS::utils::mediaplay] [AudioPlayer] Progressive streaming started - playing as we download!
[2025-12-01T22:09:26Z INFO  TempRS::utils::audio_controller] [AudioController] Audio playback started
[2025-12-01T22:09:26Z ERROR TempRS::api::tracks] [Related] API error 404 Not Found: {"code":404,"message":"","link":"https://developers.soundcloud.com/docs/api/explorer/open-api","status":"404 - Not Found","errors":[],"error":null}
[2025-12-01T22:09:26Z ERROR TempRS::data::home_data] [Home] Failed to fetch related tracks: API returned status: 404 Not Found, falling back
[2025-12-01T22:09:26Z INFO  TempRS::data::home_data] [Home] Sending 0 recommendations total
[2025-12-01T22:09:26Z INFO  TempRS::app::player_app] [Home] Received 0 recommended tracks
```

---

## Expected Behavior
✔ API should return a **valid list of related tracks (200 OK)**  
✔ Home page should populate recommended tracks

## Actual Behavior
❌ SoundCloud endpoint returned `404 Not Found`  
❌ Home page received **0 track recommendations**

---

## Hypothesis / Possible Causes
### 🔍 API Changes
- SoundCloud **deprecated or moved** the related tracks endpoint  
- Public API may no longer support `/related` without OAuth

### 🔍 Credentials
- Invalid or missing `client_id`  
- Request requires OAuth token instead of public key

### 🔍 Rate Limits / Geo-blocks
- IP temporarily blocked  
- Endpoint not available in some regions

### 🔍 Wrong URL
- Pattern may have changed:
  - `/tracks/{id}/related`  
  - `/tracks/{id}/related?client_id=XXX`  
  - `/tracks/{id}/related/sounds`

---

## Debugging Plan (Next Workday)
1. **Reproduce with cURL**
   ```sh
   curl -i \
  -H "Authorization: Bearer <ACCESS_TOKEN>" \
  "https://api.soundcloud.com/tracks/<TRACK_ID>/related"
   ```

2. **Verify current API docs**
   - Check SoundCloud dev page  
   - Cross-check unofficial API trackers

3. **Check what TempRS is calling**
   - Print full request URL  
   - Log full headers  
   - Log parsed JSON before failing

4. **If endpoint is dead → implement fallback**
   - Use similar-track logic:
     - BPM  
     - Genre  
     - Key  
     - Tags  
     - Artist followers  
   - TempRS local recommender (fast & safe)

---

## Proposed Fixes
### Short-Term
- Add detection: if 404 → don’t log error as fatal  
- Provide 6–12 fallback recommendations

### Mid-Term
- Update API route  
- Add OAuth with auto-refresh  
- Detect API schema automatically

### Long-Term
- Implement **TempRS Smart Recommendations Engine**  
- No dependency on SoundCloud related endpoint

---

## Status
⏳ **Pending investigation**  
📌 Will continue tomorrow.



### Problem Summary
- SoundCloud returns a **valid track** but with `"streamable": false` or `"policy": "BLOCK"`.
- Home screen treats this as a **fatal error** and stops loading recommendations.
- “Not playable” tracks should be **skipped**, not treated as critical failures.

### Expected Behavior
- If a track is not playable:
  - Log it
  - Skip it
  - Continue fetching the rest of the Home data

### Fix Needed
- Modify `fetch_track_full()` logic:
  ```rust
  if !track.streamable {
      warn!("Skipping non-playable track: {}", track.title);
      return Ok(None); // instead of Err(...)
  }
