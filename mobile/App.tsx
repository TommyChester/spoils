import { StatusBar } from 'expo-status-bar';
import { StyleSheet, Text, View, Button, ActivityIndicator } from 'react-native';
import { useState, useEffect } from 'react';
import { api } from './api/spoils';

export default function App() {
  const [message, setMessage] = useState<string>('');
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string>('');

  const checkHealth = async () => {
    setLoading(true);
    setError('');
    try {
      const response = await api.health();
      setMessage(`API Status: ${response.status} - ${response.message}`);
    } catch (err) {
      setError('Failed to connect to API. Make sure the backend is running on port 8080.');
    } finally {
      setLoading(false);
    }
  };

  const sayHello = async () => {
    setLoading(true);
    setError('');
    try {
      const response = await api.hello();
      setMessage(response.message);
    } catch (err) {
      setError('Failed to connect to API. Make sure the backend is running on port 8080.');
    } finally {
      setLoading(false);
    }
  };

  return (
    <View style={styles.container}>
      <Text style={styles.title}>Spoils</Text>
      <Text style={styles.subtitle}>Rust + Expo Monorepo</Text>

      {loading && <ActivityIndicator size="large" color="#0000ff" />}

      {message && <Text style={styles.message}>{message}</Text>}
      {error && <Text style={styles.error}>{error}</Text>}

      <View style={styles.buttonContainer}>
        <Button title="Check API Health" onPress={checkHealth} />
        <View style={styles.buttonSpacer} />
        <Button title="Say Hello" onPress={sayHello} />
      </View>

      <StatusBar style="auto" />
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#fff',
    alignItems: 'center',
    justifyContent: 'center',
    padding: 20,
  },
  title: {
    fontSize: 32,
    fontWeight: 'bold',
    marginBottom: 8,
  },
  subtitle: {
    fontSize: 16,
    color: '#666',
    marginBottom: 40,
  },
  message: {
    fontSize: 16,
    color: '#28a745',
    marginVertical: 20,
    textAlign: 'center',
  },
  error: {
    fontSize: 14,
    color: '#dc3545',
    marginVertical: 20,
    textAlign: 'center',
  },
  buttonContainer: {
    marginTop: 20,
    width: '100%',
  },
  buttonSpacer: {
    height: 12,
  },
});
