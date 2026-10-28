# SPECIFICATION: MANUAL QA PLAYBOOK & ACCEPTANCE TEST MATRIX
Spec ID: `[SPEC-QA-001]`
Status: APPROVED
Domain: Quality Assurance & Acceptance Criteria

---

## 1. Overview
Defines manual verification procedures, cross-platform acceptance test cases, and diagnostic check scripts to ensure hardware stability, audio synchronization, and zero memory leaks before any release packaging.

---

## 2. Test Cases `[REQ-QA-001]`

### Test Matrix

| ID | Test Name | Procedure | Expected Result |
| :--- | :--- | :--- | :--- |
| `TC-QA-01` | **Full Screen 60FPS Video Capture** | Record primary monitor for 60 seconds with 60 FPS motion test video playing. | Output MP4 video plays back at constant 60 FPS, 0 stutter, valid PTS. |
| `TC-QA-02` | **Audio/Video Sync (A/V Sync)** | Record countdown video with synchronized 1kHz audio beeps. | Video flash and audio beep align within $\pm 20\text{ms}$. |
| `TC-QA-03` | **Dual-Channel Loopback & Mic Mixing** | Play background music and speak into microphone simultaneously. | Both audio streams mixed with clear balance, no clipping or distortion. |
| `TC-QA-04` | **Window Exclusion Test** | Open floating HUD and recording window. Record desktop. | Recording HUD and control window do not appear in output video. |
| `TC-QA-05` | **Instant Replay Export** | Trigger Instant Replay after 45s of activity. | A 30s video file is saved within 1.5s, starting cleanly on a keyframe. |
| `TC-QA-06` | **Abrupt Exit RAII Cleanup** | Terminate process via Task Manager / SIGINT during active capture. | Hardware GPU sessions, COM interfaces, and temp files cleanly released, 0 locks remaining. |
| `TC-QA-07` | **Bilingual Language Toggle** | Switch language between English and Spanish. | Every single label, HUD button, error popup updates instantaneously with 0 untranslated text. |
| `TC-QA-08` | **Installer Packaging Verification** | Run packaging script and install on clean VM/machine. | Installer completes, shortcuts created, uninstaller cleanly removes app. |

---

## 3. Non-Functional Criteria `[NFR-QA-001]`
1. Zero memory leaks: RAM consumption after 1 hour of recording must be within 5% of RAM consumption at minute 2.
2. Zero dropped frames under standard 1080p60 workloads.

---

## 4. Verification Sign-Off `[TEST-QA-001]`
Every release candidate must have all test cases in this matrix checked and signed off by the Lead Architect.
