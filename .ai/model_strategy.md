# 📡 AI Model Strategy & Routing Architecture

## Profile Fallback Chains
- 👑 Boss: Gemini 3.5 Flash ➔ Nemotron 3 Ultra 550B (Nvidia)
- 💻 Code: Qwen Coder ➔ Kimi 2.6 (Nvidia) ➔ DeepSeek V4 ➔ Mistral Large
- 🐞 Debug: DeepSeek R1 ➔ Nemotron 3 Ultra ➔ Kimi 2.6
- 💬 Chat: Open Fallback Chain (Gemini ➔ Nemotron ➔ Mistral ➔ Gemma)

## Operational Parameters
- Boss Context: Hard-capped at 262,144 tokens to align with Nemotron's native limit and prevent context-cliff crashes on fallback.
- Temperatures: Boss (0.3) | Code (0.2) | Debug (0.15) | Chat (0.5)

## System Rationale
This multi-provider architecture utilizes local routing to maximize free/low-cost API tiers while maintaining enterprise-grade token depth and reasoning capacity. Do not alter mode boundaries without structural review.
