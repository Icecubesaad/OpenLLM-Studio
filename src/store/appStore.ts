import { create } from 'zustand';
import type { Model, Conversation, Message, LoadedModel, HardwareInfo, OptimizationSettings, HfDownloadProgress } from '../lib/types';
import type { ToastMessage } from '../components/ToastContainer';
import { DEFAULT_OPENROUTER_MODEL } from '../lib/cloudModels';

interface AppState {
  // Ollama connection
  ollamaConnected: boolean;
  ollamaVersion: string | null;
  
  // Models
  installedModels: Model[];
  loadedModel: LoadedModel | null;
  
  // Conversations
  conversations: Conversation[];
  currentConversationId: string | null;
  messages: Message[];
  
  // UI state
  isStreaming: boolean;
  streamingContent: string;
  toasts: ToastMessage[];
  
  // Hardware and optimization
  hardwareInfo: HardwareInfo | null;
  optimizationSettings: OptimizationSettings | null;
  
  // HuggingFace
  hfToken: string | null;
  hfUsername: string | null;
  recommendationApiKey: string | null;
  hfDownloads: Map<string, HfDownloadProgress>;
  
  // External AI API Keys
  openRouterApiKey: string | null;
  claudeApiKey: string | null;
  openAiApiKey: string | null;
  
  // Selected AI models for wizard
  wizardProvider: 'openrouter' | 'claude' | 'openai';
  openRouterModel: string;
  claudeModel: string;
  openAiModel: string;
  
  // Actions
  setOllamaConnected: (connected: boolean) => void;
  setOllamaVersion: (version: string | null) => void;
  setInstalledModels: (models: Model[]) => void;
  setLoadedModel: (model: LoadedModel | null) => void;
  setConversations: (conversations: Conversation[]) => void;
  selectConversation: (id: string | null) => void;
  setMessages: (messages: Message[]) => void;
  addMessage: (message: Message) => void;
  setIsStreaming: (isStreaming: boolean) => void;
  appendStreamingContent: (content: string) => void;
  clearStreamingContent: () => void;
  setHardwareInfo: (info: HardwareInfo | null) => void;
  setOptimizationSettings: (settings: OptimizationSettings | null) => void;
  setHfToken: (token: string | null) => void;
  setHfUsername: (username: string | null) => void;
  setRecommendationApiKey: (apiKey: string | null) => void;
  setOpenRouterApiKey: (apiKey: string | null) => void;
  setClaudeApiKey: (apiKey: string | null) => void;
  setOpenAiApiKey: (apiKey: string | null) => void;
  setWizardProvider: (provider: 'openrouter' | 'claude' | 'openai') => void;
  setOpenRouterModel: (model: string) => void;
  setClaudeModel: (model: string) => void;
  setOpenAiModel: (model: string) => void;
  updateHfDownload: (modelName: string, progress: HfDownloadProgress) => void;
  removeHfDownload: (modelName: string) => void;
  addToast: (message: string, type: ToastMessage['type']) => void;
  removeToast: (id: string) => void;
}

export const useAppStore = create<AppState>((set) => ({
  // Initial state
  ...(function hydrateApiState() {
    const legacyRecommendationApiKey = localStorage.getItem('openllm-recommendation-api-key');
    const openRouterApiKey = localStorage.getItem('openllm-openrouter-api-key') || legacyRecommendationApiKey;
    const claudeApiKey = localStorage.getItem('openllm-claude-api-key');
    const openAiApiKey = localStorage.getItem('openllm-openai-api-key');
    const savedWizardProvider = localStorage.getItem('openllm-wizard-provider') as 'openrouter' | 'claude' | 'openai' | null;
    const savedOpenRouterModel = localStorage.getItem('openllm-openrouter-model');

    return {
      recommendationApiKey: legacyRecommendationApiKey,
      openRouterApiKey,
      claudeApiKey,
      openAiApiKey,
      openRouterModel:
        savedOpenRouterModel === 'google/gemma-4-27b-it:free'
          ? DEFAULT_OPENROUTER_MODEL
          : savedOpenRouterModel || DEFAULT_OPENROUTER_MODEL,
      wizardProvider:
        savedWizardProvider ||
        (openRouterApiKey ? 'openrouter' : claudeApiKey ? 'claude' : 'openai'),
    };
  })(),
  ollamaConnected: false,
  ollamaVersion: null,
  installedModels: [],
  loadedModel: null,
  conversations: [],
  currentConversationId: null,
  messages: [],
  isStreaming: false,
  streamingContent: '',
  toasts: [],
  hardwareInfo: null,
  optimizationSettings: null,
  hfToken: localStorage.getItem('openllm-hf-token'),
  hfUsername: localStorage.getItem('openllm-hf-username'),
  hfDownloads: new Map(),
  claudeModel: localStorage.getItem('openllm-claude-model') || 'claude-sonnet-4-6',
  openAiModel: localStorage.getItem('openllm-openai-model') || 'gpt-5.4',
  
  // Actions
  setOllamaConnected: (connected) => set({ ollamaConnected: connected }),
  
  setOllamaVersion: (version) => set({ ollamaVersion: version }),
  
  setInstalledModels: (models) => set({ installedModels: models }),
  
  setLoadedModel: (model) => set({ loadedModel: model }),
  
  setConversations: (conversations) => set({ conversations }),
  
  selectConversation: (id) => set({ currentConversationId: id, messages: [], streamingContent: '' }),
  
  setMessages: (messages) => set({ messages }),
  
  addMessage: (message) => set((state) => ({ messages: [...state.messages, message] })),
  
  setIsStreaming: (isStreaming) => set({ isStreaming }),
  
  appendStreamingContent: (content) => set((state) => ({ streamingContent: state.streamingContent + content })),
  
  clearStreamingContent: () => set({ streamingContent: '' }),
  
  setHardwareInfo: (info) => set({ hardwareInfo: info }),
  
  setOptimizationSettings: (settings) => set({ optimizationSettings: settings }),
  
  setHfToken: (token) => {
    if (token) {
      localStorage.setItem('openllm-hf-token', token);
    } else {
      localStorage.removeItem('openllm-hf-token');
    }
    set({ hfToken: token });
  },
  
  setHfUsername: (username) => {
    if (username) {
      localStorage.setItem('openllm-hf-username', username);
    } else {
      localStorage.removeItem('openllm-hf-username');
    }
    set({ hfUsername: username });
  },

  setRecommendationApiKey: (apiKey) => {
    if (apiKey) {
      localStorage.setItem('openllm-recommendation-api-key', apiKey);
    } else {
      localStorage.removeItem('openllm-recommendation-api-key');
    }
    set({ recommendationApiKey: apiKey });
  },

  setOpenRouterApiKey: (apiKey) => {
    if (apiKey) {
      localStorage.setItem('openllm-openrouter-api-key', apiKey);
      localStorage.setItem('openllm-recommendation-api-key', apiKey);
    } else {
      localStorage.removeItem('openllm-openrouter-api-key');
      localStorage.removeItem('openllm-recommendation-api-key');
    }
    set({ openRouterApiKey: apiKey, recommendationApiKey: apiKey });
  },

  setClaudeApiKey: (apiKey) => {
    if (apiKey) {
      localStorage.setItem('openllm-claude-api-key', apiKey);
    } else {
      localStorage.removeItem('openllm-claude-api-key');
    }
    set({ claudeApiKey: apiKey });
  },

  setOpenAiApiKey: (apiKey) => {
    if (apiKey) {
      localStorage.setItem('openllm-openai-api-key', apiKey);
    } else {
      localStorage.removeItem('openllm-openai-api-key');
    }
    set({ openAiApiKey: apiKey });
  },

  setWizardProvider: (provider) => {
    localStorage.setItem('openllm-wizard-provider', provider);
    set({ wizardProvider: provider });
  },

  setOpenRouterModel: (model) => {
    localStorage.setItem('openllm-openrouter-model', model);
    set({ openRouterModel: model });
  },

  setClaudeModel: (model) => {
    localStorage.setItem('openllm-claude-model', model);
    set({ claudeModel: model });
  },

  setOpenAiModel: (model) => {
    localStorage.setItem('openllm-openai-model', model);
    set({ openAiModel: model });
  },
  
  updateHfDownload: (modelName, progress) => set((state) => {
    const newDownloads = new Map(state.hfDownloads);
    newDownloads.set(modelName, progress);
    return { hfDownloads: newDownloads };
  }),
  
  removeHfDownload: (modelName) => set((state) => {
    const newDownloads = new Map(state.hfDownloads);
    newDownloads.delete(modelName);
    return { hfDownloads: newDownloads };
  }),
  
  addToast: (message, type) => set((state) => ({
    toasts: [...state.toasts, { id: Date.now().toString(), message, type }],
  })),
  
  removeToast: (id) => set((state) => ({
    toasts: state.toasts.filter((toast) => toast.id !== id),
  })),
}));
