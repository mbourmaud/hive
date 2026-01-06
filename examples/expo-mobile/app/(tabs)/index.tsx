import { View, Text, StyleSheet, TouchableOpacity, ScrollView } from 'react-native';
import { useState, useEffect } from 'react';
import Button from '@/components/Button';

export default function HomeScreen() {
  const [currentTime, setCurrentTime] = useState(new Date());
  const [todayTasks, setTodayTasks] = useState(8);
  const [completedTasks, setCompletedTasks] = useState(5);

  useEffect(() => {
    const timer = setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  const getGreeting = () => {
    const hour = currentTime.getHours();
    if (hour < 12) return '🌅 Good Morning';
    if (hour < 18) return '☀️ Good Afternoon';
    return '🌙 Good Evening';
  };

  const progress = todayTasks > 0 ? (completedTasks / todayTasks) * 100 : 0;

  return (
    <ScrollView style={styles.container}>
      {/* Header with greeting */}
      <View style={styles.greetingContainer}>
        <Text style={styles.greeting}>{getGreeting()}</Text>
        <Text style={styles.timeText}>
          {currentTime.toLocaleTimeString('en-US', {
            hour: '2-digit',
            minute: '2-digit'
          })}
        </Text>
      </View>

      <Text style={styles.title}>Welcome to Expo Mobile</Text>
      <Text style={styles.subtitle}>
        Your productivity dashboard
      </Text>

      {/* Today's Progress */}
      <View style={styles.progressCard}>
        <View style={styles.progressHeader}>
          <Text style={styles.progressTitle}>Today's Progress</Text>
          <Text style={styles.progressPercentage}>{Math.round(progress)}%</Text>
        </View>
        <View style={styles.progressBarContainer}>
          <View style={[styles.progressBar, { width: `${progress}%` }]} />
        </View>
        <Text style={styles.progressText}>
          {completedTasks} of {todayTasks} tasks completed
        </Text>
      </View>

      {/* Quick Stats */}
      <View style={styles.statsGrid}>
        <View style={[styles.statCard, { backgroundColor: '#007AFF' }]}>
          <Text style={styles.statIcon}>📋</Text>
          <Text style={styles.statValue}>{todayTasks}</Text>
          <Text style={styles.statLabel}>Total Tasks</Text>
        </View>
        <View style={[styles.statCard, { backgroundColor: '#34C759' }]}>
          <Text style={styles.statIcon}>✅</Text>
          <Text style={styles.statValue}>{completedTasks}</Text>
          <Text style={styles.statLabel}>Completed</Text>
        </View>
        <View style={[styles.statCard, { backgroundColor: '#FF9500' }]}>
          <Text style={styles.statIcon}>⏰</Text>
          <Text style={styles.statValue}>{todayTasks - completedTasks}</Text>
          <Text style={styles.statLabel}>Remaining</Text>
        </View>
      </View>

      {/* Quick Actions */}
      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Quick Actions</Text>
        <View style={styles.actionsGrid}>
          <TouchableOpacity style={styles.actionButton}>
            <Text style={styles.actionIcon}>➕</Text>
            <Text style={styles.actionText}>Add Task</Text>
          </TouchableOpacity>
          <TouchableOpacity style={styles.actionButton}>
            <Text style={styles.actionIcon}>📊</Text>
            <Text style={styles.actionText}>Analytics</Text>
          </TouchableOpacity>
          <TouchableOpacity style={styles.actionButton}>
            <Text style={styles.actionIcon}>🔔</Text>
            <Text style={styles.actionText}>Reminders</Text>
          </TouchableOpacity>
          <TouchableOpacity style={styles.actionButton}>
            <Text style={styles.actionIcon}>⚙️</Text>
            <Text style={styles.actionText}>Settings</Text>
          </TouchableOpacity>
        </View>
      </View>

      {/* Features Section */}
      <View style={styles.section}>
        <Text style={styles.sectionTitle}>✨ Features</Text>
        <View style={styles.featureList}>
          <View style={styles.featureItem}>
            <Text style={styles.featureBullet}>🎨</Text>
            <Text style={styles.feature}>Modern tab navigation</Text>
          </View>
          <View style={styles.featureItem}>
            <Text style={styles.featureBullet}>📱</Text>
            <Text style={styles.feature}>Item list with categories & favorites</Text>
          </View>
          <View style={styles.featureItem}>
            <Text style={styles.featureBullet}>👤</Text>
            <Text style={styles.feature}>User profile with stats</Text>
          </View>
          <View style={styles.featureItem}>
            <Text style={styles.featureBullet}>🔍</Text>
            <Text style={styles.feature}>Real-time search & filtering</Text>
          </View>
        </View>
      </View>

      <Button
        title="Get Started"
        onPress={() => console.log('Get Started pressed')}
      />

      <View style={styles.bottomSpacer} />
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#F5F5F5',
  },
  greetingContainer: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: 20,
    paddingBottom: 8,
  },
  greeting: {
    fontSize: 20,
    fontWeight: '600',
    color: '#1A1A1A',
  },
  timeText: {
    fontSize: 16,
    fontWeight: '500',
    color: '#007AFF',
  },
  title: {
    fontSize: 28,
    fontWeight: 'bold',
    color: '#1A1A1A',
    marginBottom: 8,
    paddingHorizontal: 20,
  },
  subtitle: {
    fontSize: 16,
    color: '#666666',
    marginBottom: 20,
    paddingHorizontal: 20,
  },
  progressCard: {
    backgroundColor: '#FFFFFF',
    borderRadius: 16,
    padding: 20,
    marginHorizontal: 20,
    marginBottom: 20,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 8,
    elevation: 3,
  },
  progressHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
  },
  progressTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: '#1A1A1A',
  },
  progressPercentage: {
    fontSize: 24,
    fontWeight: 'bold',
    color: '#007AFF',
  },
  progressBarContainer: {
    height: 8,
    backgroundColor: '#E5E5E5',
    borderRadius: 4,
    marginBottom: 8,
    overflow: 'hidden',
  },
  progressBar: {
    height: '100%',
    backgroundColor: '#007AFF',
    borderRadius: 4,
  },
  progressText: {
    fontSize: 14,
    color: '#666666',
  },
  statsGrid: {
    flexDirection: 'row',
    paddingHorizontal: 20,
    marginBottom: 20,
    gap: 12,
  },
  statCard: {
    flex: 1,
    borderRadius: 12,
    padding: 16,
    alignItems: 'center',
  },
  statIcon: {
    fontSize: 28,
    marginBottom: 8,
  },
  statValue: {
    fontSize: 24,
    fontWeight: 'bold',
    color: '#FFFFFF',
    marginBottom: 4,
  },
  statLabel: {
    fontSize: 12,
    color: '#FFFFFF',
    opacity: 0.9,
  },
  section: {
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 16,
    marginHorizontal: 20,
    marginBottom: 20,
  },
  sectionTitle: {
    fontSize: 18,
    fontWeight: '600',
    color: '#1A1A1A',
    marginBottom: 12,
  },
  actionsGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 12,
  },
  actionButton: {
    flex: 1,
    minWidth: '45%',
    backgroundColor: '#F5F5F5',
    borderRadius: 12,
    padding: 16,
    alignItems: 'center',
    borderWidth: 1,
    borderColor: '#E5E5E5',
  },
  actionIcon: {
    fontSize: 32,
    marginBottom: 8,
  },
  actionText: {
    fontSize: 14,
    fontWeight: '500',
    color: '#1A1A1A',
  },
  featureList: {
    gap: 12,
  },
  featureItem: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 12,
  },
  featureBullet: {
    fontSize: 20,
  },
  feature: {
    fontSize: 14,
    color: '#666666',
    flex: 1,
  },
  bottomSpacer: {
    height: 20,
  },
});
