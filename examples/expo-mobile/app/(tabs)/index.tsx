import { View, Text, StyleSheet } from 'react-native';
import Button from '@/components/Button';

export default function HomeScreen() {
  return (
    <View style={styles.container}>
      <Text style={styles.title}>Welcome to Expo Mobile</Text>
      <Text style={styles.subtitle}>
        This is a demo app for testing with Hive drones
      </Text>

      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Features</Text>
        <Text style={styles.feature}>- Tab navigation</Text>
        <Text style={styles.feature}>- Item list with cards</Text>
        <Text style={styles.feature}>- User profile</Text>
      </View>

      <Button
        title="Get Started"
        onPress={() => console.log('Get Started pressed')}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    padding: 20,
    backgroundColor: '#F5F5F5',
  },
  title: {
    fontSize: 28,
    fontWeight: 'bold',
    color: '#1A1A1A',
    marginBottom: 8,
  },
  subtitle: {
    fontSize: 16,
    color: '#666666',
    marginBottom: 32,
  },
  section: {
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 16,
    marginBottom: 24,
  },
  sectionTitle: {
    fontSize: 18,
    fontWeight: '600',
    color: '#1A1A1A',
    marginBottom: 12,
  },
  feature: {
    fontSize: 14,
    color: '#666666',
    marginBottom: 4,
  },
});
