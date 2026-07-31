"use client";

import type { LoginRequest, RegisterRequest, User } from "@lumiforum/types";
import { useQueryClient } from "@tanstack/react-query";
import { createContext, useContext, useEffect, useState, type ReactNode } from "react";

import { getMe, login, logout, register } from "@/lib/api/auth";
import {
  clearAccessToken,
  sessionAccessToken,
  setAccessToken,
  subscribeSession,
} from "@/lib/auth/session";

type AuthStatus = "loading" | "authenticated" | "unauthenticated";

interface AuthContextValue {
  status: AuthStatus;
  user: User | null;
  signIn: (input: LoginRequest) => Promise<void>;
  signUp: (input: RegisterRequest) => Promise<void>;
  signOut: () => Promise<void>;
  restoreSession: () => Promise<User>;
  setCurrentUser: (user: User) => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();
  const [status, setStatus] = useState<AuthStatus>("loading");
  const [user, setUser] = useState<User | null>(null);

  useEffect(() => {
    let active = true;
    const unsubscribe = subscribeSession((hasAccessToken) => {
      if (!hasAccessToken && active) {
        setUser(null);
        setStatus("unauthenticated");
        queryClient.removeQueries({ queryKey: ["auth"] });
      }
    });

    void getMe()
      .then((currentUser) => {
        if (active) {
          setUser(currentUser);
          setStatus("authenticated");
          queryClient.setQueryData(["auth", "me"], currentUser);
        }
      })
      .catch(() => {
        if (active) {
          clearAccessToken();
          setStatus("unauthenticated");
        }
      });

    return () => {
      active = false;
      unsubscribe();
    };
  }, [queryClient]);

  const commitUser = (currentUser: User) => {
    setUser(currentUser);
    setStatus("authenticated");
    queryClient.setQueryData(["auth", "me"], currentUser);
  };

  const signIn = async (input: LoginRequest) => {
    const response = await login(input);
    setAccessToken(response.access_token, response.expires_in);
    commitUser(response.user);
  };

  const signUp = async (input: RegisterRequest) => {
    const response = await register(input);
    setAccessToken(response.access_token, response.expires_in);
    commitUser(response.user);
  };

  const restoreSession = async () => {
    await sessionAccessToken(true);
    const currentUser = await getMe();
    commitUser(currentUser);
    return currentUser;
  };

  const signOut = async () => {
    try {
      await logout();
    } finally {
      clearAccessToken();
      setUser(null);
      setStatus("unauthenticated");
      queryClient.removeQueries({ queryKey: ["auth"] });
    }
  };

  return (
    <AuthContext.Provider
      value={{ status, user, signIn, signUp, signOut, restoreSession, setCurrentUser: commitUser }}
    >
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error("useAuth must be used within AuthProvider");
  }
  return context;
}
