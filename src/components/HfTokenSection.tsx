import { useState } from 'react';

interface HfTokenSectionProps {
  token: string | null;
  username: string | null;
  onValidate: (token: string) => Promise<void>;
  onClear: () => void;
  isValidating: boolean;
}

export function HfTokenSection({ token, username, onValidate, onClear, isValidating }: HfTokenSectionProps) {
  const [tokenInput, setTokenInput] = useState(token || '');
  const [error, setError] = useState<string | null>(null);

  const handleValidate = async () => {
    if (!tokenInput.trim()) {
      setError('Please enter a token');
      return;
    }

    setError(null);
    try {
      await onValidate(tokenInput);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to validate token');
    }
  };

  const handleClear = () => {
    setTokenInput('');
    setError(null);
    onClear();
  };

  return (
    <div className="glass rounded-2xl p-6">
      <div className="flex items-center gap-4 mb-6">
        <div className="w-12 h-12 rounded-xl bg-[#1F1F1F] flex items-center justify-center border border-[#333333]">
          <svg className="w-6 h-6 text-[#ffcc00]" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 0C5.373 0 0 5.373 0 12s5.373 12 12 12 12-5.373 12-12S18.627 0 12 0zm-.5 4.5h1v7h-1v-7zm0 9h1v2h-1v-2z"/>
          </svg>
        </div>
        <div>
          <h3 className="text-base font-medium text-white">HuggingFace</h3>
          <p className="text-sm text-[#B1ADA1]">Optional for gated or private models</p>
        </div>
      </div>

      <div className="space-y-4">
        {/* Status */}
        <div className="flex items-center justify-between py-3 px-4 bg-[#1F1F1F] rounded-xl border border-[#2A2A2A]">
          <span className="text-sm text-[#B1ADA1]">Status</span>
          {username ? (
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 rounded-full bg-[#5a9a6e]"></div>
              <span className="text-sm text-[#5a9a6e]">Connected as {username}</span>
            </div>
          ) : (
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 rounded-full bg-[#B1ADA1]"></div>
              <span className="text-sm text-[#B1ADA1]">Not connected</span>
            </div>
          )}
        </div>

        {/* Token Input */}
        <div>
          <label className="text-sm text-[#B1ADA1] block mb-2">API Token</label>
          <div className="relative">
            <input
              type="password"
              value={tokenInput}
              onChange={(e) => setTokenInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && tokenInput.trim() && !isValidating) {
                  handleValidate();
                }
              }}
              placeholder="hf_..."
              disabled={isValidating}
              className="w-full px-4 py-3 bg-[#1F1F1F] border border-[#333333] rounded-xl text-white placeholder-[#B1ADA1] focus:outline-none focus:border-[#C15F3C] transition-all disabled:opacity-50"
            />
          </div>
          <p className="text-xs text-[#B1ADA1] mt-2">
            Required for gated models.{' '}
            <a
              href="https://huggingface.co/settings/tokens"
              target="_blank"
              rel="noopener noreferrer"
              className="text-[#C15F3C] hover:underline"
            >
              Create a token →
            </a>
          </p>
        </div>

        {/* Error */}
        {error && (
          <div className="px-4 py-3 bg-[#1F1F1F] border border-[#c45c5c] rounded-xl">
            <p className="text-[#c45c5c] text-sm">{error}</p>
          </div>
        )}

        {/* Buttons */}
        <div className="flex gap-3">
          <button
            onClick={handleValidate}
            disabled={isValidating || !tokenInput.trim()}
            className="flex-1 px-4 py-2.5 bg-[#C15F3C] hover:bg-[#A84E2F] text-white rounded-xl text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
          >
            {isValidating ? (
              <>
                <svg className="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                Validating...
              </>
            ) : (
              'Connect'
            )}
          </button>
          
          {username && (
            <button
              onClick={handleClear}
              disabled={isValidating}
              className="px-4 py-2.5 bg-[#1F1F1F] hover:bg-[#2A2A2A] border border-[#333333] text-white rounded-xl text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Disconnect
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
