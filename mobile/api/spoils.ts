const API_URL = process.env.EXPO_PUBLIC_API_URL || 'http://localhost:8080';

export interface HealthResponse {
  status: string;
  message: string;
}

export const api = {
  async health(): Promise<HealthResponse> {
    const response = await fetch(`${API_URL}/health`);
    if (!response.ok) {
      throw new Error('Health check failed');
    }
    return response.json();
  },

  async hello(): Promise<{ message: string }> {
    const response = await fetch(`${API_URL}/api/hello`);
    if (!response.ok) {
      throw new Error('API call failed');
    }
    return response.json();
  },
};
