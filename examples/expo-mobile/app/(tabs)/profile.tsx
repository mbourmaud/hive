import { View, Text, StyleSheet, Image } from 'react-native';
import Button from '@/components/Button';

export default function ProfileScreen() {
  return (
    <View style={styles.container}>
      <View style={styles.avatarContainer}>
        <View style={styles.avatar}>
          <Text style={styles.avatarText}>JD</Text>
        </View>
      </View>

      <Text style={styles.name}>John Doe</Text>
      <Text style={styles.email}>john.doe@example.com</Text>

      <View style={styles.statsContainer}>
        <View style={styles.stat}>
          <Text style={styles.statValue}>42</Text>
          <Text style={styles.statLabel}>Items</Text>
        </View>
        <View style={styles.stat}>
          <Text style={styles.statValue}>128</Text>
          <Text style={styles.statLabel}>Points</Text>
        </View>
        <View style={styles.stat}>
          <Text style={styles.statValue}>15</Text>
          <Text style={styles.statLabel}>Days</Text>
        </View>
      </View>

      <View style={styles.actions}>
        <Button
          title="Edit Profile"
          onPress={() => console.log('Edit Profile pressed')}
        />
        <Button
          title="Settings"
          onPress={() => console.log('Settings pressed')}
          variant="secondary"
        />
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#F5F5F5',
    alignItems: 'center',
    paddingTop: 40,
  },
  avatarContainer: {
    marginBottom: 16,
  },
  avatar: {
    width: 100,
    height: 100,
    borderRadius: 50,
    backgroundColor: '#007AFF',
    alignItems: 'center',
    justifyContent: 'center',
  },
  avatarText: {
    fontSize: 36,
    fontWeight: 'bold',
    color: '#FFFFFF',
  },
  name: {
    fontSize: 24,
    fontWeight: 'bold',
    color: '#1A1A1A',
    marginBottom: 4,
  },
  email: {
    fontSize: 16,
    color: '#666666',
    marginBottom: 24,
  },
  statsContainer: {
    flexDirection: 'row',
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 20,
    marginHorizontal: 20,
    marginBottom: 32,
  },
  stat: {
    flex: 1,
    alignItems: 'center',
  },
  statValue: {
    fontSize: 24,
    fontWeight: 'bold',
    color: '#007AFF',
  },
  statLabel: {
    fontSize: 14,
    color: '#666666',
    marginTop: 4,
  },
  actions: {
    width: '100%',
    paddingHorizontal: 20,
    gap: 12,
  },
});
