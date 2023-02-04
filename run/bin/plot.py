import pandas as pd
import numpy as np
import matplotlib.pyplot as plt

df0 = pd.read_csv("../benchresults/benches-pack.csv")

df = df0[ (df0['compression_ratio'] < 0.5) & (df0['time_ratio'] < 0.5) ]
compression_time = np.array(df['ns']/(1e+6))

compression_ratio = np.array(df['compression_ratio'])

compression_name = np.array(df['bench'])

for i in range(len(compression_name)):
    compression_name[i] = compression_name[i].replace('compression/', '').replace('.pack', '')

# plt.yscale('log')
# plt.xticks(rotation = 45)
plt.xticks(np.arange(0,0.4,0.05))
plt.ylabel("Time (ms)")
plt.xlabel("Compression ratio")

plt.scatter(compression_ratio, compression_time)

# add label to each data point
#for i in range(len(compression_name)):
#    plt.text(compression_time[i], compression_ratio[i]+1, compression_name)

# # zip joins x and y coordinates in pairs
for x,y,label in zip(compression_ratio, compression_time, compression_name):
    plt.annotate(label.split('/')[1], # this is the text
                 (x,y), # these are the coordinates to position the label
                 textcoords="offset points", # how to position the text
                 xytext=(0,10), # distance from text to points (x,y)
                 ha='center') # horizontal alignment can be left, right or center

plt.savefig('pack.png')
