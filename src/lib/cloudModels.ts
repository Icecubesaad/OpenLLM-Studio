// Cloud model definitions for OpenRouter, Claude, and OpenAI providers

export type CloudProvider = 'openrouter' | 'claude' | 'openai';

export interface CloudModel {
  id: string;
  name: string;
  provider: CloudProvider;
  description: string;
  contextWindow?: string;
}

export const DEFAULT_OPENROUTER_MODEL = 'google/gemma-4-26b-a4b-it:free';

export const OPENROUTER_MODELS: CloudModel[] = [
  { id: 'google/gemma-4-26b-a4b-it:free', name: 'Gemma 4 26B A4B IT (Free)', provider: 'openrouter', description: 'Google Gemma 4 26B A4B — Free tier' },
  { id: 'google/gemma-4-31b-it:free', name: 'Gemma 4 31B IT (Free)', provider: 'openrouter', description: 'Google Gemma 4 31B — Free tier' },
  { id: 'arcee-ai/trinity-large-preview:free', name: 'Trinity Large Preview (Free)', provider: 'openrouter', description: 'Arcee AI Trinity Large — Free tier' },
  { id: 'meta-llama/llama-4-maverick:free', name: 'Llama 4 Maverick (Free)', provider: 'openrouter', description: 'Meta Llama 4 Maverick — Free tier' },
  { id: 'deepseek/deepseek-r1:free', name: 'DeepSeek R1 (Free)', provider: 'openrouter', description: 'DeepSeek R1 reasoning model — Free tier' },
  { id: 'qwen/qwen3-235b-a22b:free', name: 'Qwen 3 235B (Free)', provider: 'openrouter', description: 'Qwen 3 235B — Free tier' },
  { id: 'microsoft/phi-4:free', name: 'Phi-4 (Free)', provider: 'openrouter', description: 'Microsoft Phi-4 — Free tier' },
  { id: 'google/gemini-2.5-pro-exp-03-25:free', name: 'Gemini 2.5 Pro (Free)', provider: 'openrouter', description: 'Google Gemini 2.5 Pro — Free tier' },
  { id: 'anthropic/claude-sonnet-4', name: 'Claude Sonnet 4', provider: 'openrouter', description: 'Anthropic Claude Sonnet 4 via OpenRouter' },
  { id: 'openai/gpt-4o', name: 'GPT-4o', provider: 'openrouter', description: 'OpenAI GPT-4o via OpenRouter' },
];

export const CLAUDE_MODELS: CloudModel[] = [
  { id: 'claude-sonnet-4-6', name: 'Claude Sonnet 4.6', provider: 'claude', description: 'Balanced intelligence and speed', contextWindow: '1M tokens' },
  { id: 'claude-haiku-4-5-20251001', name: 'Claude Haiku 4.5', provider: 'claude', description: 'Fast and cost-efficient', contextWindow: '200K tokens' },
  { id: 'claude-opus-4-6', name: 'Claude Opus 4.6', provider: 'claude', description: 'Maximum reasoning capability', contextWindow: '1M tokens' },
];

export const OPENAI_MODELS: CloudModel[] = [
  { id: 'gpt-5-latest', name: 'GPT-5 Latest', provider: 'openai', description: 'OpenAI flagship model' },
  { id: 'gpt-5.4', name: 'GPT-5.4', provider: 'openai', description: 'Maximum capability' },
  { id: 'gpt-5.4-mini', name: 'GPT-5.4 Mini', provider: 'openai', description: 'Cost-efficient variant' },
];

export function getAvailableCloudModels(
  openRouterKey: string | null,
  claudeKey: string | null,
  openAiKey: string | null
): CloudModel[] {
  const models: CloudModel[] = [];
  if (openRouterKey) models.push(...OPENROUTER_MODELS);
  if (claudeKey) models.push(...CLAUDE_MODELS);
  if (openAiKey) models.push(...OPENAI_MODELS);
  return models;
}

export function isCloudModel(modelId: string): CloudProvider | null {
  for (const m of OPENROUTER_MODELS) {
    if (m.id === modelId) return 'openrouter';
  }
  for (const m of CLAUDE_MODELS) {
    if (m.id === modelId) return 'claude';
  }
  for (const m of OPENAI_MODELS) {
    if (m.id === modelId) return 'openai';
  }
  return null;
}
