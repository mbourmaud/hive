import { Tabs } from 'expo-router';
import { StatusBar } from 'expo-status-bar';

export default function RootLayout() {
  return (
    <>
      <StatusBar style="auto" />
      <Tabs
        screenOptions={{
          tabBarActiveTintColor: '#007AFF',
          tabBarInactiveTintColor: '#8E8E93',
          headerStyle: {
            backgroundColor: '#FFFFFF',
          },
          headerTitleStyle: {
            fontWeight: '600',
          },
        }}
      >
        <Tabs.Screen
          name="(tabs)/index"
          options={{
            title: 'Home',
            tabBarLabel: 'Home',
          }}
        />
        <Tabs.Screen
          name="(tabs)/list"
          options={{
            title: 'Items',
            tabBarLabel: 'Items',
          }}
        />
        <Tabs.Screen
          name="(tabs)/profile"
          options={{
            title: 'Profile',
            tabBarLabel: 'Profile',
          }}
        />
      </Tabs>
    </>
  );
}
