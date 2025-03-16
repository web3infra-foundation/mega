# Performance test for FUSE.
#!/usr/bin/env python3
import os
import time
import random
import argparse
import csv
import subprocess
from multiprocessing import Pool, cpu_count
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np

class FusePerfTester:
    def __init__(self, test_path, file_size='1M', output='report'):
        self.test_path = test_path
        self.file_size = self.parse_size(file_size)
        self.output_dir = output
        self.stats = {
            'write': {'time': [], 'speed': []},
            'read': {'time': [], 'speed': []},
            'random_read': {'iops': []},
            'metadata': {'create': [], 'delete': [], 'stat': []}
        }
        os.makedirs(self.output_dir, exist_ok=True)

    @staticmethod
    def parse_size(size_str):
        units = {"B": 1, "K": 2**10, "M": 2**20, "G": 2**30}
        num = int(''.join(filter(str.isdigit, size_str)))
        unit = ''.join(filter(str.isalpha, size_str.upper())) or 'B'
        return num * units[unit]

    def _cleanup(self):
        subprocess.run(['rm', '-rf', os.path.join(self.test_path, 'testfile*')], 
                      check=True)

    def sequential_write(self, iterations=10):
        for i in range(iterations):
            filename = os.path.join(self.test_path, f'testfile_write_{i}.dat')
            start = time.time()
            with open(filename, 'wb') as f:
                f.write(os.urandom(self.file_size))
            elapsed = time.time() - start
            speed = self.file_size / elapsed / (1024**2)  # MB/s
            self.stats['write']['time'].append(elapsed)
            self.stats['write']['speed'].append(speed)

    def sequential_read(self, iterations=10):
        # 
        test_file = os.path.join(self.test_path, 'testfile_read.dat')
        with open(test_file, 'wb') as f:
            f.write(os.urandom(self.file_size))

        for _ in range(iterations):
            start = time.time()
            with open(test_file, 'rb') as f:
                f.read()
            elapsed = time.time() - start
            speed = self.file_size / elapsed / (1024**2)
            self.stats['read']['time'].append(elapsed)
            self.stats['read']['speed'].append(speed)

    def random_io_test(self, block_size=4096, duration=30):
        """randoem IOPS test"""
        test_file = os.path.join(self.test_path, 'testfile_random.dat')
        file_size = 1 * 1024**3  # 1GB for random test
        with open(test_file, 'wb') as f:
            f.truncate(file_size)

        blocks = file_size // block_size
        count = 0
        start = time.time()
        while time.time() - start < duration:
            with open(test_file, 'rb+') as f:
                offset = random.randint(0, blocks-1) * block_size
                f.seek(offset)
                f.write(os.urandom(block_size))
            count += 1
        self.stats['random_read']['iops'].append(count // duration)

    def metadata_ops(self, iterations=1000):
        """meta data operation test"""
        for i in range(iterations//3):
            # File create
            start = time.time()
            with open(os.path.join(self.test_path, f'temp_{i}.dat'), 'w') as f:
                f.write('test')
            self.stats['metadata']['create'].append(time.time() - start)

            # stat 
            start = time.time()
            os.stat(os.path.join(self.test_path, f'temp_{i}.dat'))
            self.stats['metadata']['stat'].append(time.time() - start)

            # delete 
            start = time.time()
            os.remove(os.path.join(self.test_path, f'temp_{i}.dat'))
            self.stats['metadata']['delete'].append(time.time() - start)

    def concurrent_test(self, workers=cpu_count()):
        """multi-thread test"""
        test_file = os.path.join(self.test_path, 'concurrent.dat')
        file_size = 100 * 1024**2  # 100MB
        with open(test_file, 'wb') as f:
            f.truncate(file_size)

        def worker(_):
            block_size = 4096
            offset = random.randint(0, (file_size//block_size)-1) * block_size
            data = os.urandom(block_size)
            with open(test_file, 'r+b') as f:
                f.seek(offset)
                f.write(data)

        start = time.time()
        with Pool(workers) as p:
            p.map(worker, range(1000))
        elapsed = time.time() - start
        self.stats['concurrency'] = {'total_time': elapsed, 'workers': workers}

    def generate_report(self):
        """view the result"""
        sns.set(style="whitegrid")

        # throughput test
        plt.figure(figsize=(12, 6))
        write_speeds = self.stats['write']['speed']
        read_speeds = self.stats['read']['speed']
        sns.lineplot(x=range(len(write_speeds)), y=write_speeds, label='Write Speed (MB/s)')
        sns.lineplot(x=range(len(read_speeds)), y=read_speeds, label='Read Speed (MB/s)')
        plt.title('Sequential R/W Throughput')
        plt.savefig(os.path.join(self.output_dir, 'throughput.png'))
        plt.close()

        # metadata delay
        plt.figure(figsize=(10,5))
        data = [
            np.mean(self.stats['metadata']['create']) * 1000,
            np.mean(self.stats['metadata']['stat']) * 1000,
            np.mean(self.stats['metadata']['delete']) * 1000
        ]
        labels = ['Create', 'Stat', 'Delete']
        sns.barplot(x=labels, y=data)
        plt.ylabel('Latency (ms)')
        plt.title('Metadata Operation Latency')
        plt.savefig(os.path.join(self.output_dir, 'metadata.png'))
        plt.close()

        # save key data to CSV
        with open(os.path.join(self.output_dir, 'summary.csv'), 'w') as f:
            writer = csv.writer(f)
            writer.writerow(['Test Item', 'Average', 'Max', 'Min', 'Std'])
            for test in ['write', 'read']:
                data = self.stats[test]['speed']
                writer.writerow([
                    f"{test.capitalize()} Speed (MB/s)",
                    np.mean(data),
                    max(data),
                    min(data),
                    np.std(data)
                ])
            if 'concurrency' in self.stats:
                writer.writerow([
                    'Concurrency (ops/s)', 
                    1000 / self.stats['concurrency']['total_time'],
                    '', '', ''
                ])

        # clean
        self._cleanup()

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='FUSE Filesystem Performance Tester')
    parser.add_argument('path', help='FUSE mount path to test')
    parser.add_argument('-s', '--size', default='128M', 
                      help='Test file size (e.g. 1M, 512K)')
    parser.add_argument('-o', '--output', default='report', 
                      help='Output directory for reports')
    args = parser.parse_args()

    tester = FusePerfTester(args.path, args.size, args.output)
    
    print("Running sequential write test...")
    tester.sequential_write()
    
    print("Running sequential read test...")
    tester.sequential_read()
    
    print("Running random I/O test...")
    tester.random_io_test()
    
    print("Testing metadata operations...")
    tester.metadata_ops()
    
    print("Testing concurrency...")
    tester.concurrent_test()
    
    print("Generating report...")
    tester.generate_report()
